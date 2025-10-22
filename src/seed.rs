use std::ffi::CString;
use std::os::raw::c_char;
use std::panic;

use namada_sdk::bip39::{Mnemonic, MnemonicType, Language};

// Generates 12-word seed phrase.
#[no_mangle]
pub extern "C" fn generate_seed_phrase() -> *mut c_char {
    let result = panic::catch_unwind(|| {
        let mnemonic = Mnemonic::new(MnemonicType::Words12, Language::English);
        mnemonic.phrase().to_string()
    });

    let message = match result {
        Ok(s) => s,
        Err(_) => "Failed to generate seed phrase".to_string(),
    };

    CString::new(message).unwrap().into_raw()
}

// Generates 24-word seed phrase.
#[no_mangle]
pub extern "C" fn generate_seed_phrase_24() -> *mut c_char {
    let result = panic::catch_unwind(|| {
        let mnemonic = Mnemonic::new(MnemonicType::Words24, Language::English);
        mnemonic.phrase().to_string()
    });

    let message = match result {
        Ok(s) => s,
        Err(_) => "Failed to generate seed phrase".to_string(),
    };

    CString::new(message).unwrap().into_raw()
}
