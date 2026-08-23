/*
 * Reified Gson type token for Paginated<JsonElement>.
 *
 * Generic types are erased at runtime, so Gson needs an explicit
 * TypeToken to decode the parameterized Paginated envelope.
 *
 * 𝚅𝚒𝚋𝚎𝚌𝚛𝚊𝚏𝚝𝚎𝚍. with AI Agents ⓒ 2025-2026 Vetcoders
 */

package io.loct.intellij.protocol

import com.google.gson.JsonElement
import com.google.gson.reflect.TypeToken
import java.lang.reflect.Type

object PaginatedJsonType {
    val TYPE: Type = object : TypeToken<Paginated<JsonElement>>() {}.type
}
