
plugins {
    id("com.android.application")
    id("org.jetbrains.kotlin.plugin.compose")
}

// The app's version follows the client's (Cargo.toml), so a release has one version
val cargoVersion: String =
    rootProject.file("../Cargo.toml").readLines()
        .first { it.startsWith("version = ") }
        .substringAfter('"').substringBefore('"')
val (major, minor, patch) = cargoVersion.split('.').map { it.toInt() }

android {
    namespace = "io.github.zel9278.pcscrs"
    compileSdk = 37

    defaultConfig {
        applicationId = "io.github.zel9278.pcscrs"
        // The client is built for API 24 (see the aarch64-linux-android build)
        minSdk = 24
        targetSdk = 36
        versionCode = major * 1_000_000 + minor * 1_000 + patch
        versionName = cargoVersion
        ndk { abiFilters += "arm64-v8a" }
    }

    // Release signing from the environment (CI); without it the release is signed with the debug key
    val keystore = System.getenv("ANDROID_KEYSTORE")
    signingConfigs {
        if (keystore != null) {
            create("release") {
                storeFile = file(keystore)
                storePassword = System.getenv("ANDROID_KEYSTORE_PASSWORD")
                keyAlias = System.getenv("ANDROID_KEY_ALIAS")
                keyPassword = System.getenv("ANDROID_KEY_PASSWORD")
            }
        }
    }

    buildTypes {
        release {
            isMinifyEnabled = true
            proguardFiles(getDefaultProguardFile("proguard-android-optimize.txt"), "proguard-rules.pro")
            signingConfig = signingConfigs.findByName("release") ?: signingConfigs.getByName("debug")
        }
    }

    buildFeatures {
        compose = true
        aidl = true
        buildConfig = true
    }

    packaging {
        jniLibs {
            // Extract the client to nativeLibraryDir so root or the shell user can copy it out
            useLegacyPackaging = true
            // Already stripped; there may be no NDK to strip it again
            keepDebugSymbols += "**/libpcsc.so"
        }
    }

    compileOptions {
        sourceCompatibility = JavaVersion.VERSION_17
        targetCompatibility = JavaVersion.VERSION_17
    }
}

// The client binary has to be there; say where it comes from rather than building an APK without it
val checkClient by tasks.registering {
    val lib = file("src/main/jniLibs/arm64-v8a/libpcsc.so")
    doLast {
        check(lib.isFile) {
            "Missing $lib: copy the aarch64-linux-android build of pcsc-rs there (see android/README.md)"
        }
    }
}
tasks.named("preBuild") { dependsOn(checkClient) }

dependencies {
    val composeBom = platform("androidx.compose:compose-bom:2026.09.00")
    implementation(composeBom)
    implementation("androidx.compose.material3:material3")
    implementation("androidx.compose.ui:ui")
    implementation("androidx.compose.ui:ui-tooling-preview")
    implementation("androidx.activity:activity-compose:1.13.0")
    implementation("androidx.core:core-ktx:1.19.1")
    implementation("androidx.lifecycle:lifecycle-runtime-compose:2.11.0")
    implementation("org.jetbrains.kotlinx:kotlinx-coroutines-android:1.11.0")
    implementation("dev.rikka.shizuku:api:13.1.5")
    implementation("dev.rikka.shizuku:provider:13.1.5")
}
