/*
 * Loctree IntelliJ plugin build.
 *
 * Native JetBrains LSP integration for the loctree-lsp server.
 * The plugin is a sibling editor integration and is intentionally
 * NOT a member of the Rust Cargo workspace.
 *
 * 𝚅𝚒𝚋𝚎𝚌𝚛𝚊𝚏𝚝𝚎𝚍. with AI Agents ⓒ 2025-2026 Vetcoders
 */

import org.jetbrains.intellij.platform.gradle.IntelliJPlatformType
import org.jetbrains.intellij.platform.gradle.TestFrameworkType
import org.jetbrains.intellij.platform.gradle.models.ProductRelease
import org.jetbrains.intellij.platform.gradle.tasks.VerifyPluginTask.FailureLevel
import java.nio.file.Files
import java.nio.file.StandardCopyOption

plugins {
    id("java")
    id("org.jetbrains.kotlin.jvm") version "2.2.0"
    id("org.jetbrains.intellij.platform") version "2.16.0"
}

group = providers.gradleProperty("pluginGroup").get()
version = providers.gradleProperty("pluginVersion").get()
val pluginUntilBuild = providers.gradleProperty("pluginUntilBuild").orNull
    ?.takeIf { it.isNotBlank() }
val bundledLspBinaryName =
    if (System.getProperty("os.name").lowercase().contains("win")) "loctree-lsp.exe" else "loctree-lsp"
val repoRoot = layout.projectDirectory.dir("../..")
val generatedLspResources = layout.buildDirectory.dir("generated/loctree-lsp-resources")

// Bundling the loctree-lsp runtime into the plugin ZIP is DEV-ONLY, opt-in,
// and OFF by default — every release and every CI build is download-only.
//
// Why: the bundle carries exactly one binary at the unqualified resource path
// `bin/loctree-lsp`, built for whatever host ran Gradle. BinaryResolver puts
// BUNDLED ahead of cache and verified download and only distinguishes
// `.exe`/no-`.exe`, so a single-platform bundle wins on every OS — a Linux CI
// build would hand macOS users an ELF they cannot execute, with no self-repair.
// Download-only keeps the resolver on its cache → SHA256-verified download →
// PATH chain, which is fail-closed and already covered by tests.
//
// Turn it on locally with `-PbundleLsp=true` or `LOCTREE_BUNDLE_LSP=1`.
val bundleLsp = providers.gradleProperty("bundleLsp")
    .orElse(providers.environmentVariable("LOCTREE_BUNDLE_LSP"))
    .map { it.equals("true", ignoreCase = true) || it == "1" }
    .getOrElse(false)

repositories {
    mavenCentral()
    intellijPlatform {
        defaultRepositories()
    }
}

dependencies {
    intellijPlatform {
        // Compile against the current IntelliJ Platform LSP API.
        create(
            providers.gradleProperty("platformType").get(),
            providers.gradleProperty("platformVersion").get(),
        )

        pluginVerifier()
        zipSigner()
        testFramework(TestFrameworkType.Platform)
    }

    testImplementation("junit:junit:4.13.2")
}

kotlin {
    jvmToolchain(21)
    compilerOptions {
        freeCompilerArgs.add("-Xjvm-default=all")
    }
}

intellijPlatform {
    pluginConfiguration {
        id = providers.gradleProperty("pluginId").get()
        name = "Loctree"
        version = providers.gradleProperty("pluginVersion").get()

        ideaVersion {
            sinceBuild = providers.gradleProperty("pluginSinceBuild").get()
            untilBuild = provider { pluginUntilBuild }
        }
    }

    pluginVerification {
        // The native LSP API and several platform widgets continue to move
        // across IDE lines. Fail only on genuine Marketplace breakages.
        failureLevel = listOf(
            FailureLevel.COMPATIBILITY_PROBLEMS,
            FailureLevel.INVALID_PLUGIN,
            FailureLevel.MISSING_DEPENDENCIES,
        )
        ides {
            // Keep verification focused and fast in this foundation cut.
            // The descriptor targets the LSP module, so additional IDEs can
            // be added here once the first IU lane is stable.
            select {
                types = listOf(IntelliJPlatformType.IntellijIdeaUltimate)
                channels = listOf(ProductRelease.Channel.RELEASE)
                sinceBuild = providers.gradleProperty("pluginSinceBuild").get()
                // An open-ended range makes the verifier download every IU
                // release since sinceBuild (multi-GB each) — that filled the
                // 14GB CI runner disk and killed the job with no step failure.
                // Bound verification to the first IU lane; the plugin's own
                // declared compatibility (pluginConfiguration) stays open.
                untilBuild = pluginUntilBuild ?: "252.*"
            }
        }
    }

    // Signing/publishing are operator stop-points. Tokens are read from
    // the environment only when an operator chooses to run them; CI does
    // not require Marketplace secrets for build/test/verify.
    signing {
        certificateChainFile = providers.environmentVariable("CERTIFICATE_CHAIN")
            .map { file(it) }.orNull
        privateKeyFile = providers.environmentVariable("PRIVATE_KEY")
            .map { file(it) }.orNull
        password = providers.environmentVariable("PRIVATE_KEY_PASSWORD").orNull
    }

    publishing {
        token = providers.environmentVariable("PUBLISH_TOKEN").orNull
        // Marketplace channel: "default" is the stable channel every user
        // sees; anything else (e.g. "eap") is opt-in via a custom repository.
        channels = providers.environmentVariable("PUBLISH_CHANNEL")
            .map { listOf(it) }
            .orElse(listOf("default"))
    }
}

tasks {
    val prepareBundledLsp by registering {
        group = "build"
        description = "DEV ONLY (-PbundleLsp=true): copy a locally built loctree-lsp into plugin resources."

        val bundledFile = generatedLspResources.map { it.file("bin/$bundledLspBinaryName") }
        outputs.file(bundledFile)
        onlyIf("bundleLsp opt-in is enabled") { bundleLsp }

        doLast {
            val envPath = providers.environmentVariable("LOCTREE_LSP_PATH").orNull
                ?.takeIf { it.isNotBlank() }
                ?.let { file(it) }
            val sourceFile = if (envPath != null) {
                envPath
            } else {
                val process = ProcessBuilder("cargo", "build", "-p", "loctree-lsp", "--release")
                    .directory(repoRoot.asFile)
                    .inheritIO()
                    .start()
                val exitCode = process.waitFor()
                require(exitCode == 0) {
                    "cargo build -p loctree-lsp --release failed with exit code $exitCode"
                }
                repoRoot.file("target/release/$bundledLspBinaryName").asFile
            }

            require(sourceFile.isFile) {
                "loctree-lsp binary not found at ${sourceFile.absolutePath}; set LOCTREE_LSP_PATH or build it first"
            }

            val targetFile = bundledFile.get().asFile
            targetFile.parentFile.mkdirs()
            Files.copy(sourceFile.toPath(), targetFile.toPath(), StandardCopyOption.REPLACE_EXISTING)
            if (!bundledLspBinaryName.endsWith(".exe")) {
                targetFile.setExecutable(true, false)
            }
            logger.lifecycle("Bundled $bundledLspBinaryName from ${sourceFile.absolutePath}")
        }
    }

    processResources {
        // Download-only by default: no binary resource, and no empty `bin/`
        // directory left behind in the JAR either.
        if (bundleLsp) {
            dependsOn(prepareBundledLsp)
            from(generatedLspResources)
        }
    }

    buildPlugin {
        doFirst {
            if (bundleLsp) {
                logger.lifecycle(
                    "loctree-lsp is BUNDLED in this build (dev opt-in). " +
                        "The published artifact must stay download-only — do not ship this ZIP.",
                )
            } else {
                logger.lifecycle("loctree-lsp is NOT bundled (download-only build; resolver uses cache → verified download → PATH).")
            }
        }
    }

    test {
        useJUnit()
        // Lets the resolver tests assert the download-only invariant while
        // still passing under the dev-only `-PbundleLsp=true` opt-in.
        systemProperty("loctree.bundleLsp", bundleLsp.toString())
    }
}
