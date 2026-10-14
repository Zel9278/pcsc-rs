plugins {
    id("com.android.application")
}

val cargoVersion: String = rootProject.file("../Cargo.toml").readLines()
    .first { it.startsWith("version = ") }
    .substringAfter('"').substringBefore('"')
val (major, minor, patch) = cargoVersion.split('.').map { it.toInt() }

android {
    namespace = "io.github.zel9278.pcscrs.legacy"
    compileSdk = providers.gradleProperty("android.compileSdk").map { it.toInt() }.getOrElse(37)

    defaultConfig {
        applicationId = "io.github.zel9278.pcscrs.legacy"
        minSdk = 10
        // Keeps the APK installable on current Android while retaining Gingerbread APIs only.
        targetSdk = 23
        versionCode = major * 1_000_000 + minor * 1_000 + patch
        versionName = cargoVersion
    }
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
            isMinifyEnabled = false
            if (keystore != null) {
                signingConfig = signingConfigs.getByName("release")
            }
        }
    }

    lint {
        disable += "ExpiredTargetSdkVersion"
    }

    compileOptions {
        sourceCompatibility = JavaVersion.VERSION_1_8
        targetCompatibility = JavaVersion.VERSION_1_8
    }
}

// These are standalone, static client executables stored as assets; they are extracted at runtime.
tasks.register("checkLegacyClients") {
    doLast {
        listOf("armeabi", "armeabi-v7a").forEach { abi ->
            val binary = file("src/main/assets/bin/$abi/pcsc-rs")
            check(binary.isFile) { "Missing $binary; build the ARM legacy client first" }
        }
    }
}
tasks.named("preBuild") { dependsOn("checkLegacyClients") }
