# iPhone And iPad Builds

This directory contains the native Xcode host for Davenstein on iPhone and iPad

The Rust game, shared touch controls, assets, and platform-independent logic remain in the repository root. The Xcode project supplies the Apple application bundle, icon catalog, device-family metadata, and the build phase that selects the correct Rust iOS target

## Supported Build Targets

- Physical iPhone and iPad: `aarch64-apple-ios`
- Apple Silicon iPhone and iPad Simulator: `aarch64-apple-ios-sim`

The project declares `TARGETED_DEVICE_FAMILY = "1,2"` so one application supports both iPhone and iPad

## Prepare Assets

The application bundle requires a generated `assets.pak`, which is intentionally ignored by Git

```sh
ios/prepare-assets.sh
```

## Build An Unsigned Device Application

```sh
rustup target add aarch64-apple-ios

ios/prepare-assets.sh

xcodebuild \
  -project ios/Davenstein.xcodeproj \
  -scheme Davenstein \
  -configuration Release \
  -sdk iphoneos \
  -destination 'generic/platform=iOS' \
  -derivedDataPath target/ios-device \
  CODE_SIGNING_ALLOWED=NO \
  clean build
```

## Build An Unsigned Apple Silicon Simulator Application

```sh
rustup target add aarch64-apple-ios-sim

ios/prepare-assets.sh

xcodebuild \
  -project ios/Davenstein.xcodeproj \
  -scheme Davenstein \
  -configuration Release \
  -sdk iphonesimulator \
  -destination 'generic/platform=iOS Simulator' \
  -derivedDataPath target/ios-simulator \
  CODE_SIGNING_ALLOWED=NO \
  ARCHS=arm64 \
  ONLY_ACTIVE_ARCH=YES \
  clean build
```

## Local Device Signing

No personal or company Apple team is committed in the project

For private physical-device testing, supply a development team and a bundle identifier at build time. Xcode can then register and provision a connected device through `-allowProvisioningUpdates` and `-allowProvisioningDeviceRegistration`

Public App Store, TestFlight, or enterprise distribution is outside the approved Davenstein Apple distribution scope
