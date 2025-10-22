
use std::path::PathBuf;
use std::fs;
use std::env;

use std::{
    panic::{self, AssertUnwindSafe},
    backtrace::Backtrace,
};

use std::ffi::{CStr, CString};
use std::os::raw::c_char;
use std::str::FromStr;
use tokio::runtime::Builder;
use tendermint_rpc::{HttpClient, Url};
use namada_sdk::{
    args::TxBuilder,
    io::NullIo,
    masp::fs::FsShieldedUtils,
    masp::ShieldedContext,
    wallet::fs::FsWalletUtils,
    NamadaImpl,
    chain::ChainId,
    bip39::{Mnemonic, Language, MnemonicType},
    key::SchemeType,
    wallet::DerivationPath,
    zeroize::Zeroizing,
    signing::default_sign,

};
use tokio::runtime::Runtime;
use namada_sdk::args::InputAmount;
use namada_sdk::Namada; 
use namada_sdk::args::TxTransparentTransfer;
use namada_sdk::address::Address;


use namada_sdk::wallet::StoredKeypair;
use namada_core::masp::{
    ExtendedViewingKey,
};
use namada_sdk::PaymentAddress;

use masp_primitives::zip32::{ExtendedSpendingKey, ExtendedFullViewingKey};
use namada_sdk::string_encoding::Format;
use masp_primitives::zip32::DiversifierIndex;

use namada_sdk::masp::find_valid_diversifier;
use rand_core::OsRng;

// Android logging helper
#[cfg(target_os = "android")]
mod android_log {
    use std::ffi::CString;
    use std::os::raw::{c_char, c_int};
    extern "C" {
        fn __android_log_print(prio: c_int, tag: *const c_char, msg: *const c_char) -> c_int;
    }
    pub fn log_debug(tag: &str, msg: &str) {
        let t = CString::new(tag).unwrap_or_default();
        let m = CString::new(msg).unwrap_or_default();
        unsafe { __android_log_print(3, t.as_ptr(), m.as_ptr()); }
    }
}
#[cfg(not(target_os = "android"))]
mod android_log {
    pub fn log_debug(_t: &str, _m: &str) {}
}
use android_log::log_debug;

// Panic hook with backtrace
fn init_panic_logging() {
    panic::set_hook(Box::new(|info| {
        let bt = Backtrace::force_capture();
        let payload = info
            .payload()
            .downcast_ref::<&str>()
            .map(|s| *s)
            .or_else(|| info.payload().downcast_ref::<String>().map(String::as_str))
            .unwrap_or("Unknown panic payload");
        log_debug(
            "namada",
            &format!("❌ PANIC: {}\nLocation: {:?}\nBacktrace:\n{:?}", payload, info.location(), bt),
        );
    }));
}


#[no_mangle]
pub extern "C" fn derive_and_save_wallet(input: *const c_char) -> *mut c_char {
    init_panic_logging();

    let result = std::panic::catch_unwind(|| {
        log_debug("namada", "✅ starting deriving");

        if input.is_null() {
            return Err("Error: input pointer was null".into());
        }
        let c_str = unsafe { CStr::from_ptr(input) };
        let s = c_str.to_str().map_err(|e| format!("Invalid UTF-8 input: {:?}", e))?;
        let parts: Vec<&str> = s.split("::").collect();
        if parts.len() != 3 {
            return Err("Expected format wallet_path::alias::seed_phrase".into());
        }
        let wallet_path = parts[0];
        let alias = parts[1].to_string();
        let seed_phrase = parts[2];
        log_debug(
            "namada",
            &format!(
                "🔍 Parsed input: wallet_path='{}', alias='{}', seed='{}'",
                wallet_path, alias, seed_phrase
            ),
        );

        // Ensure directories exist
        fs::create_dir_all(wallet_path)
            .map_err(|e| format!("Could not create wallet dir '{}': {:?}", wallet_path, e))?;
        let masp_dir = PathBuf::from(wallet_path).join("masp");
        fs::create_dir_all(&masp_dir)
            .map_err(|e| format!("Could not create masp dir {:?}: {:?}", masp_dir, e))?;
        log_debug("namada", "✅ parsed input & dirs ok");
        log_debug(
            "namada",
            &format!(
                "🔍 maspDir='{}'",
                masp_dir.display()
            ),
        );

        std::env::set_var("NAMADA_MASP_PARAMS_DIR", format!("{}/masp", wallet_path));

        // Parse mnemonic
        let mnemonic = Mnemonic::from_phrase(seed_phrase, Language::English)
            .map_err(|e| format!("Bad mnemonic: {:?}", e))?;
        log_debug("namada", "✅ parsed mnemonic ok");

        // Build Tokio runtime
        let rt = Builder::new_current_thread()
            .enable_all()
            .build()
            .map_err(|e| format!("Failed to create runtime: {:?}", e))?;
        log_debug("namada", "✅ tokio runtime ok");

        rt.block_on(async {
            let url = Url::from_str("https://rpc.namada.tududes.com")
                .map_err(|e| format!("Bad RPC URL: {:?}", e))?;
            let http_client = HttpClient::new(url)
                .map_err(|e| format!("HTTP client error: {:?}", e))?;
            log_debug("namada", "✅ RPC client ok");

            let wallet = FsWalletUtils::new(PathBuf::from(wallet_path).join("sdk-wallet"));
            let shielded_ctx = ShieldedContext::new(FsShieldedUtils::new(PathBuf::from(wallet_path).join("masp")));
            let mut sdk = NamadaImpl::new(http_client, wallet, shielded_ctx.into(), namada_sdk::io::NullIo)
                .await
                .map_err(|e| format!("Failed to init SDK: {:?}", e))?
                .chain_id(
                    ChainId::from_str("namada.5f5de2dd1b88cba30586420")
                        .map_err(|e| format!("Bad chain ID: {:?}", e))?
                );
            log_debug("namada", "✅ SDK init ok");

            let shielded_alias = format!("{}_shielded", alias);
            let payment_alias = format!("{}_payment", shielded_alias);
            
            let mut wallet_guard = sdk.wallet_mut().await;

            // Derive key
            let opt_key = wallet_guard.derive_store_key_from_mnemonic_code(
                SchemeType::Ed25519,
                Some(alias.clone()),
                true,
                DerivationPath::default_for_transparent_scheme(SchemeType::Ed25519),
                Some((mnemonic.clone(), Zeroizing::new(String::new()))),
                false,
                None,
            );

            let _key = opt_key.ok_or_else(|| {
                format!("derive_store_key_from_mnemonic_code returned None for alias '{}'", alias)
            })?;    
            log_debug("namada", "✅ key derived ok");

            let (alias2, sdk_extsk) = wallet_guard
                .derive_store_spending_key_from_mnemonic_code(
                    shielded_alias.clone(), // alias
                    false, None, false,
                    DerivationPath::default_for_shielded(),
                    Some((mnemonic.clone(), Zeroizing::new(String::new()))),
                    false, None)
                .expect("derive shielded failed");
            
            let view_key = wallet_guard.find_viewing_key(alias2.clone()).expect("Could not get viewing key");
            log_debug("namada", &format!("✅ viewing key: {:#?}", view_key));

            let viewing_key = view_key.as_viewing_key();
            let (div, _g_d) = find_valid_diversifier(&mut OsRng);
            let masp_payment_addr = viewing_key.to_payment_address(div).expect("Unable to generate a PaymentAddress");
            let payment_addr = PaymentAddress::from(masp_payment_addr);
            wallet_guard
                .insert_payment_addr(payment_alias.to_string(), payment_addr.clone(), false) // Convert alias to String
                .expect("Payment address could not be inserted");

            // Save wallet
            wallet_guard.save()
                .map_err(|e| format!("Save error: {:?}", e))?;
            log_debug("namada", "✅ wallet saved ok");
            
            Ok::<_, String>(format!("Derived & saved wallet '{}'", alias))
        })
    });

    let out_str = match result {
        Ok(Ok(msg))        => msg,
        Ok(Err(err_msg))   => err_msg,
        Err(panic_payload) => {
            let panic_msg = panic_payload
                .downcast_ref::<&str>().map(|s| *s)
                .or_else(|| panic_payload.downcast_ref::<String>().map(String::as_str))
                .unwrap_or("unknown panic payload");
            format!("💥 Top-level panic: {}", panic_msg)
        }
    };

    // Build C string for return
    let c_out = CString::new(out_str)
        .unwrap_or_else(|_| CString::new("Internal error: null byte in output").unwrap());
    c_out.into_raw()
}






