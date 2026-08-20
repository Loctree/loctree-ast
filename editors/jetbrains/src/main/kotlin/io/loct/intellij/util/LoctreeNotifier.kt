/*
 * Balloon notification helpers for the Loctree plugin.
 *
 * 𝚅𝚒𝚋𝚎𝚌𝚛𝚊𝚏𝚝𝚎𝚍. with AI Agents ⓒ 2025-2026 Vetcoders
 */

package io.loct.intellij.util

import com.intellij.notification.NotificationGroupManager
import com.intellij.notification.NotificationType
import com.intellij.openapi.project.Project

object LoctreeNotifier {

    private const val GROUP_ID = "Loctree"

    fun info(project: Project, message: String) = notify(project, message, NotificationType.INFORMATION)

    fun warn(project: Project, message: String) = notify(project, message, NotificationType.WARNING)

    fun error(project: Project, message: String) = notify(project, message, NotificationType.ERROR)

    private fun notify(project: Project, message: String, type: NotificationType) {
        NotificationGroupManager.getInstance()
            .getNotificationGroup(GROUP_ID)
            .createNotification(message, type)
            .notify(project)
    }
}
