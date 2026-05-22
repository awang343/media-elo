package com.mediaelo.client.ui

import androidx.lifecycle.ViewModel
import androidx.lifecycle.viewModelScope
import com.mediaelo.client.api.Row
import com.mediaelo.client.data.Repo
import kotlinx.coroutines.flow.MutableStateFlow
import kotlinx.coroutines.flow.SharingStarted
import kotlinx.coroutines.flow.StateFlow
import kotlinx.coroutines.flow.combine
import kotlinx.coroutines.flow.stateIn
import kotlinx.coroutines.flow.update
import kotlinx.coroutines.launch

enum class SortKey(val label: String) {
    EloDesc("Elo (high → low)"),
    EloAsc("Elo (low → high)"),
    TitleAsc("Title (A → Z)"),
    MatchesDesc("Matches (most)"),
    DateAddedDesc("Date added (newest)"),
}

data class LibraryFilters(
    val search: String = "",
    val type: String? = null,
    val sort: SortKey = SortKey.EloDesc,
)

data class LibraryUiState(
    val loading: Boolean = true,
    val error: String? = null,
    val displayed: List<Row> = emptyList(),
    val types: List<String> = emptyList(),
    val filters: LibraryFilters = LibraryFilters(),
)

class LibraryViewModel : ViewModel() {
    private val filters = MutableStateFlow(LibraryFilters())
    private val error = MutableStateFlow<String?>(null)

    val state: StateFlow<LibraryUiState> = combine(
        Repo.rows, Repo.types, filters, error,
    ) { rows, types, f, err ->
        LibraryUiState(
            loading = rows == null && err == null,
            error = err,
            displayed = rows?.let { applyFilters(it, f) }.orEmpty(),
            types = types.orEmpty(),
            filters = f,
        )
    }.stateIn(viewModelScope, SharingStarted.Eagerly, LibraryUiState())

    init {
        if (Repo.rows.value == null) refresh()
        if (Repo.types.value == null) refreshTypes()
    }

    fun refresh() {
        error.value = null
        viewModelScope.launch {
            try {
                Repo.refresh()
                if (Repo.types.value == null) Repo.refreshTypes()
            } catch (t: Throwable) {
                error.value = t.message ?: t.javaClass.simpleName
            }
        }
    }

    private fun refreshTypes() {
        viewModelScope.launch {
            runCatching { Repo.refreshTypes() }
        }
    }

    fun setSearch(value: String) { filters.update { it.copy(search = value) } }
    fun setTypeFilter(type: String?) { filters.update { it.copy(type = type) } }
    fun setSort(sort: SortKey) { filters.update { it.copy(sort = sort) } }

    private fun applyFilters(rows: List<Row>, f: LibraryFilters): List<Row> {
        val q = f.search.trim()
        val filtered = rows.asSequence()
            .filter { f.type == null || it.type == f.type }
            .filter { q.isEmpty() || it.title.contains(q, ignoreCase = true) }
            .toList()
        return when (f.sort) {
            SortKey.EloDesc -> filtered.sortedByDescending { it.elo }
            SortKey.EloAsc -> filtered.sortedBy { it.elo }
            SortKey.TitleAsc -> filtered.sortedBy { it.title.lowercase() }
            SortKey.MatchesDesc -> filtered.sortedByDescending { it.matches }
            SortKey.DateAddedDesc -> filtered.sortedByDescending { it.dateAdded }
        }
    }
}

