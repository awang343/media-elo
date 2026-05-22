package com.mediaelo.client.ui

import androidx.compose.foundation.background
import androidx.compose.foundation.horizontalScroll
import androidx.compose.foundation.layout.Arrangement
import androidx.compose.foundation.layout.Box
import androidx.compose.foundation.layout.Column
import androidx.compose.foundation.layout.PaddingValues
import androidx.compose.foundation.layout.Row
import androidx.compose.foundation.layout.Spacer
import androidx.compose.foundation.layout.fillMaxSize
import androidx.compose.foundation.layout.fillMaxWidth
import androidx.compose.foundation.layout.height
import androidx.compose.foundation.layout.padding
import androidx.compose.foundation.lazy.LazyColumn
import androidx.compose.foundation.lazy.items
import androidx.compose.foundation.rememberScrollState
import androidx.compose.foundation.shape.RoundedCornerShape
import androidx.compose.material.icons.Icons
import androidx.compose.material.icons.automirrored.outlined.Sort
import androidx.compose.material.icons.outlined.Add
import androidx.compose.material3.Button
import androidx.compose.material3.Card
import androidx.compose.material3.CardDefaults
import androidx.compose.material3.CircularProgressIndicator
import androidx.compose.material3.DropdownMenu
import androidx.compose.material3.DropdownMenuItem
import androidx.compose.material3.ExperimentalMaterial3Api
import androidx.compose.material3.ExtendedFloatingActionButton
import androidx.compose.material3.FilterChip
import androidx.compose.material3.Icon
import androidx.compose.material3.IconButton
import androidx.compose.material3.MaterialTheme
import androidx.compose.material3.OutlinedTextField
import androidx.compose.material3.Text
import androidx.compose.runtime.Composable
import androidx.compose.runtime.collectAsState
import androidx.compose.runtime.getValue
import androidx.compose.runtime.mutableStateOf
import androidx.compose.runtime.remember
import androidx.compose.runtime.rememberCoroutineScope
import androidx.compose.runtime.saveable.rememberSaveable
import androidx.compose.runtime.setValue
import androidx.compose.ui.Alignment
import androidx.compose.ui.Modifier
import androidx.compose.ui.text.font.FontWeight
import androidx.compose.ui.unit.dp
import androidx.lifecycle.viewmodel.compose.viewModel
import com.mediaelo.client.api.Row as MediaRow
import com.mediaelo.client.data.Repo
import com.mediaelo.client.data.STATUS_DONE
import com.mediaelo.client.data.STATUS_DROPPED
import kotlinx.coroutines.launch

private sealed interface Sheet {
    data object Add : Sheet
    data class Edit(val row: MediaRow) : Sheet
}

@OptIn(ExperimentalMaterial3Api::class)
@Composable
fun LibraryScreen(
    contentPadding: PaddingValues,
    vm: LibraryViewModel = viewModel(),
) {
    val state by vm.state.collectAsState()
    var sheet by remember { mutableStateOf<Sheet?>(null) }
    val scope = rememberCoroutineScope()

    Box(modifier = Modifier.fillMaxSize().padding(contentPadding)) {
        Column(modifier = Modifier.fillMaxSize()) {
            FilterBar(
                filters = state.filters,
                types = state.types,
                onSearch = vm::setSearch,
                onTypeFilter = vm::setTypeFilter,
                onSort = vm::setSort,
            )
            Box(modifier = Modifier.fillMaxSize()) {
                when {
                    state.loading -> CircularProgressIndicator(Modifier.align(Alignment.Center))
                    state.error != null -> ErrorView(state.error!!) { vm.refresh() }
                    state.displayed.isEmpty() -> EmptyView(state.filters.search.isNotBlank() || state.filters.type != null)
                    else -> RowList(
                        rows = state.displayed,
                        onClick = { row -> sheet = Sheet.Edit(row) },
                    )
                }
            }
        }
        ExtendedFloatingActionButton(
            onClick = { sheet = Sheet.Add },
            icon = { Icon(Icons.Outlined.Add, contentDescription = null) },
            text = { Text("Add") },
            modifier = Modifier
                .align(Alignment.BottomEnd)
                .padding(16.dp),
        )
    }

    when (val s = sheet) {
        Sheet.Add -> AddSheet(
            types = state.types,
            onDismiss = { sheet = null },
        )
        is Sheet.Edit -> EditSheet(
            row = s.row,
            types = state.types,
            onDismiss = { sheet = null },
            onDeleted = {
                sheet = null
                scope.launch { runCatching { Repo.refresh() } }
            },
        )
        null -> Unit
    }
}

@OptIn(ExperimentalMaterial3Api::class)
@Composable
private fun FilterBar(
    filters: LibraryFilters,
    types: List<String>,
    onSearch: (String) -> Unit,
    onTypeFilter: (String?) -> Unit,
    onSort: (SortKey) -> Unit,
) {
    Column(modifier = Modifier.fillMaxWidth().padding(horizontal = 12.dp, vertical = 8.dp)) {
        Row(verticalAlignment = Alignment.CenterVertically) {
            OutlinedTextField(
                value = filters.search,
                onValueChange = onSearch,
                placeholder = { Text("Search title") },
                singleLine = true,
                modifier = Modifier.weight(1f),
            )
            SortMenu(current = filters.sort, onPick = onSort)
        }
        Spacer(Modifier.height(8.dp))
        Row(
            modifier = Modifier.horizontalScroll(rememberScrollState()),
            horizontalArrangement = Arrangement.spacedBy(6.dp),
        ) {
            FilterChip(
                selected = filters.type == null,
                onClick = { onTypeFilter(null) },
                label = { Text("All") },
            )
            types.forEach { t ->
                FilterChip(
                    selected = filters.type == t,
                    onClick = { onTypeFilter(if (filters.type == t) null else t) },
                    label = { Text(t) },
                )
            }
        }
    }
}

@Composable
private fun SortMenu(current: SortKey, onPick: (SortKey) -> Unit) {
    var open by rememberSaveable { mutableStateOf(false) }
    Box {
        IconButton(onClick = { open = true }) {
            Icon(Icons.AutoMirrored.Outlined.Sort, contentDescription = "Sort")
        }
        DropdownMenu(expanded = open, onDismissRequest = { open = false }) {
            SortKey.entries.forEach { key ->
                DropdownMenuItem(
                    text = {
                        Text(
                            key.label,
                            fontWeight = if (key == current) FontWeight.Bold else FontWeight.Normal,
                        )
                    },
                    onClick = { onPick(key); open = false },
                )
            }
        }
    }
}

@Composable
private fun RowList(rows: List<MediaRow>, onClick: (MediaRow) -> Unit) {
    LazyColumn(
        modifier = Modifier.fillMaxSize(),
        contentPadding = PaddingValues(horizontal = 8.dp, vertical = 4.dp),
    ) {
        items(rows, key = { it.id }) { row -> MediaCard(row, onClick) }
    }
}

@OptIn(ExperimentalMaterial3Api::class)
@Composable
private fun MediaCard(row: MediaRow, onClick: (MediaRow) -> Unit) {
    Card(
        onClick = { onClick(row) },
        modifier = Modifier.fillMaxWidth().padding(vertical = 4.dp),
        colors = CardDefaults.cardColors(
            containerColor = MaterialTheme.colorScheme.surfaceVariant,
            contentColor = MaterialTheme.colorScheme.onSurfaceVariant,
        ),
    ) {
        Row(
            modifier = Modifier.fillMaxWidth().padding(12.dp),
            verticalAlignment = Alignment.CenterVertically,
        ) {
            Column(modifier = Modifier.weight(1f)) {
                Text(row.title, fontWeight = FontWeight.SemiBold)
                Row(verticalAlignment = Alignment.CenterVertically) {
                    Text(row.type, style = MaterialTheme.typography.bodySmall)
                    Spacer(Modifier.height(0.dp).padding(horizontal = 4.dp))
                    StatusPill(row.status)
                }
            }
            Column(horizontalAlignment = Alignment.End) {
                Text("%.0f".format(row.elo), fontWeight = FontWeight.Bold)
                Text("${row.matches} matches", style = MaterialTheme.typography.bodySmall)
            }
        }
    }
}

@Composable
private fun StatusPill(status: String) {
    val (bg, fg) = when (status) {
        STATUS_DONE -> MaterialTheme.colorScheme.primary to MaterialTheme.colorScheme.onPrimary
        STATUS_DROPPED -> MaterialTheme.colorScheme.error to MaterialTheme.colorScheme.onError
        else -> MaterialTheme.colorScheme.secondary to MaterialTheme.colorScheme.onSecondary
    }
    Box(
        modifier = Modifier
            .padding(start = 6.dp)
            .background(bg, RoundedCornerShape(50))
            .padding(horizontal = 8.dp, vertical = 2.dp),
    ) {
        Text(status, color = fg, style = MaterialTheme.typography.labelSmall)
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
        Spacer(Modifier.height(8.dp))
        Text(message, style = MaterialTheme.typography.bodySmall)
        Spacer(Modifier.height(16.dp))
        Button(onClick = onRetry) { Text("Retry") }
    }
}

@Composable
private fun EmptyView(filtered: Boolean) {
    Box(modifier = Modifier.fillMaxSize(), contentAlignment = Alignment.Center) {
        Text(if (filtered) "No matches" else "Library is empty")
    }
}
