plugins {
    // Lets Gradle auto-provision the JDK 21 toolchain required by modern IDE targets.
    id("org.gradle.toolchains.foojay-resolver-convention") version "1.0.0"
}

rootProject.name = "loctree-intellij"
