# namada-sdk-wrapper-keychain

Wrapper for Namada SDK.

Mobile app uses FFI to call functions Namada SDK wrapper, so you need to compile and build static/dynamic library firstly and add them to main repo [namada-mobile-wallet].

## 1. Compile native library for different platforms

### Android (physical device and emulator)

arm64
```bash
rustup target add aarch64-linux-android
cargo ndk --target aarch64-linux-android build --release
```
x86_64
```bash
rustup target add x86_64-linux-android
cargo ndk --target x86_64-linux-android build --release
```

### iOS (physical device)

arm64
```bash
rustup target add aarch64-apple-ios
cargo build --target aarch64-apple-ios --release
```
x86_64
```bash
rustup target add x86_64-apple-ios
cargo build --target x86_64-apple-ios --release
```

### iOS Simulator

arm64
```bash
rustup target add aarch64-apple-ios-sim
cargo build --target aarch64-apple-ios-sim --release
```
x86_64
```bash
rustup target add x86_64-apple-ios-sim
cargo build --target x86_64-apple-ios-sim --release
```


## 2. Add compiled library to **namada-mobile-wallet-keychain**

### Android
- `android../jniLibs/arm64-v8a/libnamada_wrapper.so`
- `android../jniLibs/x86_64/libnamada_wrapper.so`

### iOS
- `ios../RustLib/libnamada_wrapper.a`


