package com.mediaelo.client.ui

import androidx.compose.foundation.layout.Arrangement
import androidx.compose.foundation.layout.Column
import androidx.compose.foundation.layout.PaddingValues
import androidx.compose.foundation.layout.Row
import androidx.compose.foundation.layout.fillMaxSize
import androidx.compose.foundation.layout.fillMaxWidth
import androidx.compose.foundation.layout.padding
import androidx.compose.foundation.text.KeyboardOptions
import androidx.compose.material3.Button
import androidx.compose.material3.MaterialTheme
import androidx.compose.material3.OutlinedButton
import androidx.compose.material3.OutlinedTextField
import androidx.compose.material3.Text
import androidx.compose.runtime.Composable
import androidx.compose.runtime.collectAsState
import androidx.compose.runtime.getValue
import androidx.compose.ui.Modifier
import androidx.compose.ui.text.input.KeyboardType
import androidx.compose.ui.unit.dp
import androidx.lifecycle.viewmodel.compose.viewModel

@Composable
fun SettingsScreen(
    contentPadding: PaddingValues,
    vm: SettingsViewModel = viewModel(),
) {
    val state by vm.state.collectAsState()
    Column(
        modifier = Modifier.fillMaxSize().padding(contentPadding).padding(16.dp),
        verticalArrangement = Arrangement.spacedBy(12.dp),
    ) {
        Text("Server", style = MaterialTheme.typography.titleMedium)
        Text(
            "URL the app talks to. Use http://10.0.2.2:7878 from the Android " +
                "emulator, your LAN IP from a physical device, or 127.0.0.1 " +
                "after `adb reverse tcp:7878 tcp:7878`.",
            style = MaterialTheme.typography.bodySmall,
            color = MaterialTheme.colorScheme.onSurfaceVariant,
        )

        OutlinedTextField(
            value = state.draft,
            onValueChange = vm::onDraftChange,
            label = { Text("Base URL") },
            placeholder = { Text("http://192.168.x.x:7878") },
            singleLine = true,
            isError = state.error != null,
            keyboardOptions = KeyboardOptions(keyboardType = KeyboardType.Uri),
            supportingText = {
                when {
                    state.error != null -> Text(state.error!!)
                    state.justSaved -> Text("Saved")
                    else -> Text("Current: ${state.saved}")
                }
            },
            modifier = Modifier.fillMaxWidth(),
        )

        Row(
            modifier = Modifier.fillMaxWidth(),
            horizontalArrangement = Arrangement.spacedBy(8.dp),
        ) {
            Button(
                onClick = vm::save,
                enabled = state.dirty && state.error == null,
                modifier = Modifier.weight(1f),
            ) { Text("Save") }
            OutlinedButton(
                onClick = vm::resetToDefault,
                modifier = Modifier.weight(1f),
            ) { Text("Reset") }
        }
    }
}
