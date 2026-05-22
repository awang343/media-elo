package com.mediaelo.client.data

import com.mediaelo.client.api.MediaEloClient
import com.mediaelo.client.api.Row
import com.mediaelo.client.api.UndoRequest
import com.mediaelo.client.api.VoteResponse
import kotlinx.coroutines.flow.MutableStateFlow
import kotlinx.coroutines.flow.StateFlow
import kotlinx.coroutines.flow.asStateFlow
import kotlinx.coroutines.flow.update

/**
 * Single source of truth for the row list shared by all screens.
 * After a vote, the server returns the two updated rows; we splice them in
 * so the library view stays consistent without a round-trip refresh.
 *
 * The HTTP layer reads the server URL from [Settings] on every request,
 * so changing it in the Settings screen takes effect on the next call.
 */
object Repo {
    private val client = MediaEloClient { Settings.baseUrl.value }

    private val _rows = MutableStateFlow<List<Row>?>(null)
    val rows: StateFlow<List<Row>?> = _rows.asStateFlow()

    suspend fun refresh() {
        _rows.value = client.listRows()
    }

    /** Drop cached rows so the next screen entry fetches fresh from the new server. */
    fun invalidate() {
        _rows.value = null
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
