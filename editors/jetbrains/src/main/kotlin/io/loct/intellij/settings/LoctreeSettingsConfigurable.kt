/*
 * Settings UI for the Loctree plugin (Tools > Loctree).
 *
 * 𝚅𝚒𝚋𝚎𝚌𝚛𝚊𝚏𝚝𝚎𝚍. with AI Agents ⓒ 2025-2026 Vetcoders
 */

package io.loct.intellij.settings

import com.intellij.openapi.options.BoundConfigurable
import com.intellij.openapi.ui.DialogPanel
import com.intellij.ui.dsl.builder.bindItem
import com.intellij.ui.dsl.builder.bindSelected
import com.intellij.ui.dsl.builder.bindText
import com.intellij.ui.dsl.builder.columns
import com.intellij.ui.dsl.builder.panel
import com.intellij.ui.dsl.builder.toNullableProperty
import io.loct.intellij.LoctreeBundle

class LoctreeSettingsConfigurable :
    BoundConfigurable(LoctreeBundle.message("settings.display.name")) {

    private val settings = LoctreeSettingsState.getInstance()

    override fun createPanel(): DialogPanel = panel {
        group(LoctreeBundle.message("settings.section.runtime")) {
            row(LoctreeBundle.message("settings.serverPath.label")) {
                textField()
                    .bindText(settings::serverPath)
                    .comment(LoctreeBundle.message("settings.serverPath.comment"))
                    .columns(40)
            }
        }

        group(LoctreeBundle.message("settings.section.behavior")) {
            row {
                checkBox(LoctreeBundle.message("settings.autoRefresh"))
                    .bindSelected(settings::autoRefresh)
            }
            row {
                checkBox(LoctreeBundle.message("settings.showStatusBar"))
                    .bindSelected(settings::showStatusBar)
            }
            row(LoctreeBundle.message("settings.diagnosticSeverity.label")) {
                comboBox(DiagnosticSeverity.entries)
                    .bindItem(settings::diagnosticSeverity.toNullableProperty())
            }
        }

        group(LoctreeBundle.message("settings.section.download")) {
            row {
                checkBox(LoctreeBundle.message("settings.autoDownload"))
                    .bindSelected(settings::autoDownload)
            }
            row(LoctreeBundle.message("settings.downloadBaseUrl.label")) {
                textField()
                    .bindText(settings::downloadBaseUrl)
                    .comment(LoctreeBundle.message("settings.downloadBaseUrl.comment"))
                    .columns(40)
            }
            row(LoctreeBundle.message("settings.downloadTag.label")) {
                textField()
                    .bindText(settings::downloadTag)
                    .comment(LoctreeBundle.message("settings.downloadTag.comment"))
                    .columns(20)
            }
        }
    }
}
