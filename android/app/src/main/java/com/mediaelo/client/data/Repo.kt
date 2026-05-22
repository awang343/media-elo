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
 * 10.0.2.2 = emulator host loopback. For a physical device, run
 *   adb reverse tcp:7878 tcp:7878
 * and reach the host via 127.0.0.1.
 */
object Repo {
    private const val BASE_URL = "http://10.0.2.2:7878"

    private val client = MediaEloClient(BASE_URL)

    private val _rows = MutableStateFlow<List<Row>?>(null)
    val rows: StateFlow<List<Row>?> = _rows.asStateFlow()

    suspend fun refresh() {
        _rows.value = client.listRows()
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
