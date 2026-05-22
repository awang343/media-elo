package com.mediaelo.client.data

import com.mediaelo.client.api.AddRequest
import com.mediaelo.client.api.EditRequest
import com.mediaelo.client.api.MediaEloClient
import com.mediaelo.client.api.Row
import com.mediaelo.client.api.UndoRequest
import com.mediaelo.client.api.VoteResponse
import kotlinx.coroutines.flow.MutableStateFlow
import kotlinx.coroutines.flow.StateFlow
import kotlinx.coroutines.flow.asStateFlow
import kotlinx.coroutines.flow.update

/**
 * Single source of truth for rows + types, shared by all screens.
 * Mutating endpoints splice the server's response into the local lists so
 * dependent screens (Library, Vote) update without an extra round-trip.
 *
 * The HTTP layer reads the server URL from [Settings] on every request,
 * so changing it in the Settings screen takes effect on the next call.
 */
object Repo {
    private val client = MediaEloClient { Settings.baseUrl.value }

    private val _rows = MutableStateFlow<List<Row>?>(null)
    val rows: StateFlow<List<Row>?> = _rows.asStateFlow()

    private val _types = MutableStateFlow<List<String>?>(null)
    val types: StateFlow<List<String>?> = _types.asStateFlow()

    suspend fun refresh() {
        _rows.value = client.listRows()
    }

    suspend fun refreshTypes() {
        _types.value = client.listTypes()
    }

    /** Drop cached rows + types so the next screen entry refetches from the new server. */
    fun invalidate() {
        _rows.value = null
        _types.value = null
    }

    suspend fun addRow(req: AddRequest): Row {
        val row = client.addRow(req)
        _rows.update { it?.plus(row) }
        return row
    }

    suspend fun editRow(id: String, req: EditRequest): Row {
        val updated = client.editRow(id, req)
        _rows.update { current -> current?.map { if (it.id == id) updated else it } }
        return updated
    }

    suspend fun deleteRow(id: String) {
        client.deleteRow(id)
        _rows.update { current -> current?.filter { it.id != id } }
    }

    suspend fun vote(winnerId: String, loserId: String): VoteResponse {
        val resp = client.vote(winnerId, loserId)
        _rows.update { current ->
            current?.map {
                when (it.id) {
                    resp.winner.id -> resp.winner
                    resp.loser.id -> resp.loser
                    else -> it
                }
            }
        }
        return resp
    }

    suspend fun undo(req: UndoRequest) {
        client.undo(req)
        _rows.update { current ->
            current?.map { row ->
                when (row.id) {
                    req.aId -> row.copy(elo = req.oldEloA, matches = req.oldMatchesA)
                    req.bId -> row.copy(elo = req.oldEloB, matches = req.oldMatchesB)
                    else -> row
                }
            }
        }
    }
}
