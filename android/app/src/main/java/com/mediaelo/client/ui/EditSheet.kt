package com.mediaelo.client.ui

import androidx.compose.foundation.layout.Arrangement
import androidx.compose.foundation.layout.Column
import androidx.compose.foundation.layout.Row
import androidx.compose.foundation.layout.fillMaxWidth
import androidx.compose.foundation.layout.imePadding
import androidx.compose.foundation.layout.navigationBarsPadding
import androidx.compose.foundation.layout.padding
import androidx.compose.material3.AlertDialog
import androidx.compose.material3.Button
import androidx.compose.material3.ButtonDefaults
import androidx.compose.material3.ExperimentalMaterial3Api
import androidx.compose.material3.MaterialTheme
import androidx.compose.material3.ModalBottomSheet
import androidx.compose.material3.OutlinedButton
import androidx.compose.material3.OutlinedTextField
import androidx.compose.material3.Text
import androidx.compose.material3.TextButton
import androidx.compose.material3.rememberModalBottomSheetState
import androidx.compose.runtime.Composable
import androidx.compose.runtime.getValue
import androidx.compose.runtime.mutableStateOf
import androidx.compose.runtime.remember
import androidx.compose.runtime.rememberCoroutineScope
import androidx.compose.runtime.setValue
import androidx.compose.ui.Modifier
import androidx.compose.ui.unit.dp
import com.mediaelo.client.api.EditRequest
import com.mediaelo.client.api.Row
import com.mediaelo.client.data.Repo
import com.mediaelo.client.data.STATUSES
import kotlinx.coroutines.launch

@OptIn(ExperimentalMaterial3Api::class)
@Composable
fun EditSheet(
    row: Row,
    types: List<String>,
    onDismiss: () -> Unit,
    onDeleted: () -> Unit,
) {
    val sheetState = rememberModalBottomSheetState(skipPartiallyExpanded = true)
    val scope = rememberCoroutineScope()

    var title by remember { mutableStateOf(row.title) }
    var type by remember { mutableStateOf(row.type) }
    var status by remember { mutableStateOf(row.status) }
    var error by remember { mutableStateOf<String?>(null) }
    var saving by remember { mutableStateOf(false) }
    var confirmDelete by remember { mutableStateOf(false) }

    ModalBottomSheet(onDismissRequest = onDismiss, sheetState = sheetState) {
        Column(
            modifier = Modifier
                .fillMaxWidth()
                .navigationBarsPadding()
                .imePadding()
                .padding(horizontal = 16.dp, vertical = 8.dp),
            verticalArrangement = Arrangement.spacedBy(12.dp),
        ) {
            Text("Edit", style = MaterialTheme.typography.titleLarge)
            Text(
                "elo ${"%.0f".format(row.elo)} • ${row.matches} matches",
                style = MaterialTheme.typography.bodySmall,
                color = MaterialTheme.colorScheme.onSurfaceVariant,
            )

            OutlinedTextField(
                value = title,
                onValueChange = { title = it; error = null },
                label = { Text("Title") },
                singleLine = true,
                modifier = Modifier.fillMaxWidth(),
            )

            DropdownPicker(
                label = "Type",
                options = types.ifEmpty { listOf(row.type) },
                selected = type,
                onSelect = { type = it },
            )

            DropdownPicker(
                label = "Status",
                options = STATUSES,
                selected = status,
                onSelect = { status = it },
            )

            error?.let {
                Text(it, color = MaterialTheme.colorScheme.error, style = MaterialTheme.typography.bodySmall)
            }

            Row(
                modifier = Modifier.fillMaxWidth(),
                horizontalArrangement = Arrangement.spacedBy(8.dp),
            ) {
                OutlinedButton(
                    onClick = { confirmDelete = true },
                    colors = ButtonDefaults.outlinedButtonColors(
                        contentColor = MaterialTheme.colorScheme.error,
                    ),
                    modifier = Modifier.weight(1f),
                    enabled = !saving,
                ) { Text("Delete") }

                OutlinedButton(
                    onClick = { scope.launch { sheetState.hide(); onDismiss() } },
                    modifier = Modifier.weight(1f),
                    enabled = !saving,
                ) { Text("Cancel") }

                Button(
                    onClick = {
                        val trimmedTitle = title.trim()
                        if (trimmedTitle.isEmpty()) { error = "Title is required"; return@Button }
                        if (type.isBlank()) { error = "Type is required"; return@Button }
                        saving = true
                        scope.launch {
                            try {
                                Repo.editRow(row.id, EditRequest(
                                    type = type,
                                    title = trimmedTitle,
                                    status = status,
                                ))
                                sheetState.hide()
                                onDismiss()
                            } catch (t: Throwable) {
                                error = t.message ?: "Failed to save"
                            } finally {
                                saving = false
                            }
                        }
                    },
                    enabled = !saving,
                    modifier = Modifier.weight(1f),
                ) { Text(if (saving) "Saving…" else "Save") }
            }
        }
    }

    if (confirmDelete) {
        AlertDialog(
            onDismissRequest = { confirmDelete = false },
            title = { Text("Delete row?") },
            text = { Text("\"${row.title}\" will be removed from the library.") },
            confirmButton = {
                TextButton(onClick = {
                    confirmDelete = false
                    saving = true
                    scope.launch {
                        try {
                            Repo.deleteRow(row.id)
                            sheetState.hide()
                            onDeleted()
                        } catch (t: Throwable) {
                            error = t.message ?: "Failed to delete"
                        } finally {
                            saving = false
                        }
                    }
                }) { Text("Delete", color = MaterialTheme.colorScheme.error) }
            },
            dismissButton = {
                TextButton(onClick = { confirmDelete = false }) { Text("Cancel") }
            },
        )
    }
}
