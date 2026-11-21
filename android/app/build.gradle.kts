plugins {
    id("com.android.application")
    id("org.jetbrains.kotlin.android")
    id("org.jetbrains.kotlin.plugin.compose")
}

// CI nightlies: `-Pnightly=true -PversionCode=… -PversionName=…`, signed with
// the key in SIGNING_STORE_FILE / SIGNING_STORE_PASSWORD / SIGNING_KEY_ALIAS /
// SIGNING_KEY_PASSWORD. The nightly is its own app (eu.depau.quoquendo.nightly).
val nightly = (findProperty("nightly") as String?) == "true"
val signingStore = System.getenv("SIGNING_STORE_FILE")?.takeIf { it.isNotEmpty() }

android {
    namespace = "eu.depau.quoquendo"
    compileSdk = 35

    defaultConfig {
        applicationId = "eu.depau.quoquendo"
        minSdk = 26
        targetSdk = 35
        versionCode = (findProperty("versionCode") as String?)?.toInt() ?: 1
        versionName = (findProperty("versionName") as String?) ?: "0.1.0"
        manifestPlaceholders["appName"] = "quoquendo"
        val mirror = (System.getenv("QQ_MIRROR_URL") ?: "").trimEnd('/')
        buildConfigField("String", "QQ_MIRROR_URL", "\"$mirror\"")
        ndk {
            abiFilters += listOf("arm64-v8a", "armeabi-v7a", "x86_64")
        }
        if (nightly) {
            applicationIdSuffix = ".nightly"
            manifestPlaceholders["appName"] = "quoquendo Nightly"
        }
    }

    signingConfigs {
        if (signingStore != null) {
            create("ci") {
                storeFile = file(signingStore)
                storePassword = System.getenv("SIGNING_STORE_PASSWORD")
                keyAlias = System.getenv("SIGNING_KEY_ALIAS")
                keyPassword = System.getenv("SIGNING_KEY_PASSWORD")
            }
        }
    }

    buildTypes {
        release {
            isMinifyEnabled = false
            signingConfigs.findByName("ci")?.let { signingConfig = it }
        }
    }
    compileOptions {
        sourceCompatibility = JavaVersion.VERSION_17
        targetCompatibility = JavaVersion.VERSION_17
    }
    kotlinOptions {
        jvmTarget = "17"
    }
    buildFeatures {
        compose = true
        buildConfig = true
    }
}

// The engine: libquoquendo_jni.so for every ABI, built by cargo-ndk.
val buildRust by tasks.registering(Exec::class) {
    description = "Builds the quoquendo JNI library with cargo-ndk"
    workingDir = rootProject.projectDir
    commandLine("./build-rust.sh")
}
tasks.named("preBuild") { dependsOn(buildRust) }

dependencies {
    val composeBom = platform("androidx.compose:compose-bom:2024.10.00")
    implementation(composeBom)
    implementation("androidx.core:core-ktx:1.13.1")
    implementation("androidx.activity:activity-compose:1.9.3")
    implementation("androidx.lifecycle:lifecycle-runtime-ktx:2.8.7")
    implementation("androidx.compose.ui:ui")
    implementation("androidx.compose.material3:material3")
    implementation("androidx.compose.material:material-icons-extended")
    implementation("org.jetbrains.kotlinx:kotlinx-coroutines-android:1.8.1")
}
