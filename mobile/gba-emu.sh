cd GBAEmulatorMobile

sed -i '' "s/RustXcframework/RustXcframework2/g" Package.swift

cd Sources/GBAEmulatorMobile

sed -i '' "s/RustXcframework/RustXcframework2/g" gba-emulator-mobile.swift
sed -i '' "s/RustXcframework/RustXcframework2/g" SwiftBridgeCore.swift

cd ../..

mv RustXcframework.xcframework RustXcframework2.xcframework

cd RustXcframework2.xcframework/ios-arm64/Headers

sed -i '' "s/RustXcframework/RustXcframework2/g" module.modulemap

mkdir gba-emulator
mv gba-emulator-mobile.h ./gba-emulator/gba-emulator-mobile.h
mv module.modulemap ./gba-emulator/module.modulemap
mv SwiftBridgeCore.h ./gba-emulator/SwiftBridgeCore.h

cd ../..

cd ios-arm64_x86_64-simulator/Headers

sed -i '' "s/RustXcframework/RustXcframework2/g" module.modulemap

mkdir gba-emulator
mv gba-emulator-mobile.h ./gba-emulator/gba-emulator-mobile.h
mv module.modulemap ./gba-emulator/module.modulemap
mv SwiftBridgeCore.h ./gba-emulator/SwiftBridgeCore.h

cd ../..

cd macos-arm64_x86_64/headers

sed -i '' "s/RustXcframework/RustXcframework2/g" module.modulemap

mkdir gba-emulator
mv gba-emulator-mobile.h ./gba-emulator/gba-emulator-mobile.h
mv module.modulemap ./gba-emulator/module.modulemap
mv SwiftBridgeCore.h ./gba-emulator/SwiftBridgeCore.h

