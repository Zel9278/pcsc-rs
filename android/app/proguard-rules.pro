# Conscrypt and Bouncy Castle are looked up by name (JCA providers)
-keep class org.conscrypt.** { *; }
-keep class org.bouncycastle.** { *; }
-dontwarn org.conscrypt.**
-dontwarn org.bouncycastle.**
# Native SPAKE2 for pairing
-keep class io.github.muntashirakon.crypto.** { *; }
# Shizuku starts the user service by its class name
-keep class io.github.zel9278.pcscrs.ShellService { <init>(...); }
