package com.mediaelo.client.ui

import androidx.lifecycle.ViewModel
import androidx.lifecycle.viewModelScope
import com.mediaelo.client.api.MediaEloClient
import com.mediaelo.client.api.Row
import kotlinx.coroutines.flow.MutableStateFlow
import kotlinx.coroutines.flow.StateFlow
import kotlinx.coroutines.flow.asStateFlow
import kotlinx.coroutines.launch

sealed interface LibraryState {
    data object Loading : LibraryState
    data class Loaded(val rows: List<Row>) : LibraryState
    data class Error(val message: String) : LibraryState
}

class LibraryViewModel(
    // 10.0.2.2 = host loopback from the Android emulator.
    // For a physical device: `adb reverse tcp:7878 tcp:7878` then use 127.0.0.1.
    baseUrl: String = "http://10.0.2.2:7878",
) : ViewModel() {
    private val client = MediaEloClient(baseUrl)

    private val _state = MutableStateFlow<LibraryState>(LibraryState.Loading)
    val state: StateFlow<LibraryState> = _state.asStateFlow()

    init { refresh() }

    fun refresh() {
        _state.value = LibraryState.Loading
        viewModelScope.launch {
            _state.value = try {
                LibraryState.Loaded(client.listRows().sortedByDescending { it.elo })
            } catch (t: Throwable) {
                LibraryState.Error(t.message ?: t.javaClass.simpleName)
            }
        }
    }

    override fun onCleared() {
        client.close()
        super.onCleared()
    }
}
