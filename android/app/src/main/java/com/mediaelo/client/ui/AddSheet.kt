package com.mediaelo.client.ui

import androidx.compose.foundation.layout.Arrangement
import androidx.compose.foundation.layout.Column
import androidx.compose.foundation.layout.Row
import androidx.compose.foundation.layout.fillMaxWidth
import androidx.compose.foundation.layout.imePadding
import androidx.compose.foundation.layout.navigationBarsPadding
import androidx.compose.foundation.layout.padding
import androidx.compose.foundation.text.KeyboardOptions
import androidx.compose.material3.Button
import androidx.compose.material3.ExperimentalMaterial3Api
import androidx.compose.material3.MaterialTheme
import androidx.compose.material3.ModalBottomSheet
import androidx.compose.material3.OutlinedButton
import androidx.compose.material3.OutlinedTextField
import androidx.compose.material3.Text
import androidx.compose.material3.rememberModalBottomSheetState
import androidx.compose.runtime.Composable
import androidx.compose.runtime.getValue
import androidx.compose.runtime.mutableStateOf
import androidx.compose.runtime.remember
import androidx.compose.runtime.rememberCoroutineScope
import androidx.compose.runtime.setValue
import androidx.compose.ui.Modifier
import androidx.compose.ui.text.input.KeyboardType
import androidx.compose.ui.unit.dp
import com.mediaelo.client.api.AddRequest
import com.mediaelo.client.data.Repo
import com.mediaelo.client.data.STATUSES
import com.mediaelo.client.data.STATUS_BACKLOG
import kotlinx.coroutines.launch

@OptIn(ExperimentalMaterial3Api::class)
@Composable
fun AddSheet(
    types: List<String>,
    onDismiss: () -> Unit,
) {
    val sheetState = rememberModalBottomSheetState(skipPartiallyExpanded = true)
    val scope = rememberCoroutineScope()

    var title by remember { mutableStateOf("") }
    var type by remember { mutableStateOf(types.firstOrNull().orEmpty()) }
    var status by remember { mutableStateOf(STATUS_BACKLOG) }
    var ratingText by remember { mutableStateOf("") }
    var error by remember { mutableStateOf<String?>(null) }
    var saving by remember { mutableStateOf(false) }

    ModalBottomSheet(onDismissRequest = onDismiss, sheetState = sheetState) {
        Column(
            modifier = Modifier
                .fillMaxWidth()
                .navigationBarsPadding()
                .imePadding()
                .padding(horizontal = 16.dp, vertical = 8.dp),
            verticalArrangement = Arrangement.spacedBy(12.dp),
        ) {
            Text("Add row", style = MaterialTheme.typography.titleLarge)

            OutlinedTextField(
                value = title,
                onValueChange = { title = it; error = null },
                label = { Text("Title") },
                singleLine = true,
                modifier = Modifier.fillMaxWidth(),
            )

            if (types.isEmpty()) {
                Text(
                    "No types available. Add one on the server first.",
                    color = MaterialTheme.colorScheme.error,
                    style = MaterialTheme.typography.bodySmall,
                )
            } else {
                DropdownPicker(
                    label = "Type",
                    options = types,
                    selected = type.ifBlank { types.first() },
                    onSelect = { type = it },
                )
            }

            DropdownPicker(
                label = "Status",
                options = STATUSES,
                selected = status,
                onSelect = { status = it },
            )

            OutlinedTextField(
                value = ratingText,
                onValueChange = { ratingText = it; error = null },
                label = { Text("Rating 1–10 (optional)") },
                placeholder = { Text("blank → 1500 elo") },
                singleLine = true,
                keyboardOptions = KeyboardOptions(keyboardType = KeyboardType.Decimal),
                modifier = Modifier.fillMaxWidth(),
            )

            error?.let {
                Text(it, color = MaterialTheme.colorScheme.error, style = MaterialTheme.typography.bodySmall)
            }

            Row(
                modifier = Modifier.fillMaxWidth(),
                horizontalArrangement = Arrangement.spacedBy(8.dp),
            ) {
                OutlinedButton(
                    onClick = { scope.launch { sheetState.hide(); onDismiss() } },
                    modifier = Modifier.weight(1f),
                ) { Text("Cancel") }
                Button(
                    onClick = {
                        val trimmedTitle = title.trim()
                        if (trimmedTitle.isEmpty()) { error = "Title is required"; return@Button }
                        val chosenType = type.ifBlank { types.firstOrNull() }
                        if (chosenType.isNullOrBlank()) { error = "Type is required"; return@Button }
                        val rating = ratingText.trim().takeIf { it.isNotEmpty() }?.toDoubleOrNull()
                        if (ratingText.isNotBlank() && (rating == null || rating !in 1.0..10.0)) {
                            error = "Rating must be 1–10"; return@Button
                        }
                        saving = true
                        scope.launch {
                            try {
                                Repo.addRow(AddRequest(
                                    type = chosenType,
                                    title = trimmedTitle,
                                    rating = rating,
                                    status = status,
                                ))
                                sheetState.hide()
                                onDismiss()
                            } catch (t: Throwable) {
                                error = t.message ?: "Failed to add"
                            } finally {
                                saving = false
                            }
                        }
                    },
                    enabled = !saving && types.isNotEmpty(),
                    modifier = Modifier.weight(1f),
                ) { Text(if (saving) "Saving…" else "Save") }
            }
        }
    }
}
