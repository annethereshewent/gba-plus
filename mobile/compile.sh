rm -rf GBAEmulatorMobile
./build-rust.sh
swift-bridge-cli create-package \
--bridges-dir ./generated \
--out-dir GBAEmulatorMobile \
--ios target/aarch64-apple-ios/release/libgba_emulator_mobile.a \
--simulator target/universal-ios/release/libgba_emulator_mobile.a \
--macos target/universal-macos/release/libgba_emulator_mobile.a \
--name GBAEmulatorMobile
./gba-emu.sh