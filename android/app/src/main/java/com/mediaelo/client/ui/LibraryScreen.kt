package com.mediaelo.client.ui

import androidx.compose.foundation.layout.Arrangement
import androidx.compose.foundation.layout.Box
import androidx.compose.foundation.layout.Column
import androidx.compose.foundation.layout.Row
import androidx.compose.foundation.layout.Spacer
import androidx.compose.foundation.layout.fillMaxSize
import androidx.compose.foundation.layout.fillMaxWidth
import androidx.compose.foundation.layout.padding
import androidx.compose.foundation.layout.width
import androidx.compose.foundation.lazy.LazyColumn
import androidx.compose.foundation.lazy.items
import androidx.compose.material3.Button
import androidx.compose.material3.Card
import androidx.compose.material3.CircularProgressIndicator
import androidx.compose.material3.ExperimentalMaterial3Api
import androidx.compose.material3.MaterialTheme
import androidx.compose.material3.Scaffold
import androidx.compose.material3.Text
import androidx.compose.material3.TopAppBar
import androidx.compose.runtime.Composable
import androidx.compose.runtime.collectAsState
import androidx.compose.runtime.getValue
import androidx.compose.ui.Alignment
import androidx.compose.ui.Modifier
import androidx.compose.ui.text.font.FontWeight
import androidx.compose.ui.unit.dp
import androidx.lifecycle.viewmodel.compose.viewModel
import com.mediaelo.client.api.Row as MediaRow

@OptIn(ExperimentalMaterial3Api::class)
@Composable
fun LibraryScreen(vm: LibraryViewModel = viewModel()) {
    val state by vm.state.collectAsState()
    Scaffold(
        topBar = { TopAppBar(title = { Text("Media Elo") }) },
    ) { padding ->
        Box(modifier = Modifier.fillMaxSize().padding(padding)) {
            when (val s = state) {
                LibraryState.Loading -> CircularProgressIndicator(Modifier.align(Alignment.Center))
                is LibraryState.Error -> ErrorView(s.message) { vm.refresh() }
                is LibraryState.Loaded -> RowList(s.rows)
            }
        }
    }
}

@Composable
private fun ErrorView(message: String, onRetry: () -> Unit) {
    Column(
        modifier = Modifier.fillMaxSize().padding(24.dp),
        horizontalAlignment = Alignment.CenterHorizontally,
        verticalArrangement = Arrangement.Center,
    ) {
        Text("Could not reach server", style = MaterialTheme.typography.titleMedium)
        Spacer(Modifier.width(8.dp))
        Text(message, style = MaterialTheme.typography.bodySmall)
        Spacer(Modifier.width(16.dp))
        Button(onClick = onRetry) { Text("Retry") }
    }
}

@Composable
private fun RowList(rows: List<MediaRow>) {
    if (rows.isEmpty()) {
        Box(modifier = Modifier.fillMaxSize(), contentAlignment = Alignment.Center) {
            Text("Library is empty")
        }
        return
    }
    LazyColumn(modifier = Modifier.fillMaxSize().padding(8.dp)) {
        items(rows, key = { it.id }) { row -> MediaCard(row) }
    }
}

@Composable
private fun MediaCard(row: MediaRow) {
    Card(modifier = Modifier.fillMaxWidth().padding(vertical = 4.dp)) {
        Row(modifier = Modifier.fillMaxWidth().padding(12.dp), verticalAlignment = Alignment.CenterVertically) {
            Column(modifier = Modifier.weight(1f)) {
                Text(row.title, fontWeight = FontWeight.SemiBold)
                Text("${row.type} • ${row.status}", style = MaterialTheme.typography.bodySmall)
            }
            Column(horizontalAlignment = Alignment.End) {
                Text("%.0f".format(row.elo), fontWeight = FontWeight.Bold)
                Text("${row.matches} matches", style = MaterialTheme.typography.bodySmall)
            }
        }
    }
}
