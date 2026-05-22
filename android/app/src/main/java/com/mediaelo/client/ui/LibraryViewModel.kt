package com.mediaelo.client.ui

import androidx.lifecycle.ViewModel
import androidx.lifecycle.viewModelScope
import com.mediaelo.client.api.Row
import com.mediaelo.client.data.Repo
import kotlinx.coroutines.flow.MutableStateFlow
import kotlinx.coroutines.flow.SharingStarted
import kotlinx.coroutines.flow.StateFlow
import kotlinx.coroutines.flow.asStateFlow
import kotlinx.coroutines.flow.combine
import kotlinx.coroutines.flow.stateIn
import kotlinx.coroutines.launch

sealed interface LibraryState {
    data object Loading : LibraryState
    data class Loaded(val rows: List<Row>) : LibraryState
    data class Error(val message: String) : LibraryState
}

class LibraryViewModel : ViewModel() {
    private val error = MutableStateFlow<String?>(null)

    val state: StateFlow<LibraryState> = combine(Repo.rows, error) { rows, err ->
        when {
            err != null -> LibraryState.Error(err)
            rows == null -> LibraryState.Loading
            else -> LibraryState.Loaded(rows.sortedByDescending { it.elo })
        }
    }.stateIn(viewModelScope, SharingStarted.Eagerly, LibraryState.Loading)

    init { if (Repo.rows.value == null) refresh() }

    fun refresh() {
        error.value = null
        viewModelScope.launch {
            try {
                Repo.refresh()
            } catch (t: Throwable) {
                error.value = t.message ?: t.javaClass.simpleName
            }
        }
    }
}
