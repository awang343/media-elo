package com.mediaelo.client.ui

import androidx.lifecycle.ViewModel
import androidx.lifecycle.viewModelScope
import com.mediaelo.client.api.Row
import com.mediaelo.client.api.UndoRequest
import com.mediaelo.client.data.Pairer
import com.mediaelo.client.data.Repo
import com.mediaelo.client.data.eligibleIndices
import kotlinx.coroutines.flow.MutableStateFlow
import kotlinx.coroutines.flow.StateFlow
import kotlinx.coroutines.flow.asStateFlow
import kotlinx.coroutines.launch

data class VoteResult(
    val winnerTitle: String,
    val loserTitle: String,
    val deltaWinner: Double,
    val deltaLoser: Double,
)

private data class UndoSnapshot(
    val aId: String,
    val bId: String,
    val oldEloA: Double,
    val oldEloB: Double,
    val oldMatchesA: Int,
    val oldMatchesB: Int,
)

sealed interface VoteState {
    data object Loading : VoteState
    data class Error(val message: String) : VoteState

    /** Eligible rows exist but no valid pair could be drawn (or all in recent history). */
    data object NoPair : VoteState

    /** Not enough rankable rows to form any pair. */
    data object NotEnough : VoteState

    data class Ready(
        val left: Row,
        val right: Row,
        val lastResult: VoteResult?,
        val canUndo: Boolean,
        val voting: Boolean,
    ) : VoteState
}

class VoteViewModel : ViewModel() {
    private val pairer = Pairer()
    private val undoStack = ArrayDeque<UndoSnapshot>()
    private var lastResult: VoteResult? = null
    private var voting = false
    private var currentPair: Pair<Row, Row>? = null

    private val _state = MutableStateFlow<VoteState>(VoteState.Loading)
    val state: StateFlow<VoteState> = _state.asStateFlow()

    init {
        viewModelScope.launch {
            try {
                if (Repo.rows.value == null) Repo.refresh()
                nextPair()
            } catch (t: Throwable) {
                _state.value = VoteState.Error(t.message ?: t.javaClass.simpleName)
            }
        }
    }

    fun retry() {
        _state.value = VoteState.Loading
        viewModelScope.launch {
            try {
                Repo.refresh()
                nextPair()
            } catch (t: Throwable) {
                _state.value = VoteState.Error(t.message ?: t.javaClass.simpleName)
            }
        }
    }

    fun skip() {
        lastResult = null
        nextPair()
    }

    fun vote(leftWins: Boolean) {
        val pair = currentPair ?: return
        if (voting) return
        voting = true
        publish()

        val (a, b) = pair
        val winner = if (leftWins) a else b
        val loser = if (leftWins) b else a
        val snapshot = UndoSnapshot(
            aId = a.id, bId = b.id,
            oldEloA = a.elo, oldEloB = b.elo,
            oldMatchesA = a.matches, oldMatchesB = b.matches,
        )

        viewModelScope.launch {
            try {
                val resp = Repo.vote(winner.id, loser.id)
                pairer.remember(resp.winner, resp.loser)
                undoStack.addLast(snapshot)
                lastResult = VoteResult(
                    winnerTitle = resp.winner.title,
                    loserTitle = resp.loser.title,
                    deltaWinner = resp.deltaWinner,
                    deltaLoser = resp.deltaLoser,
                )
                voting = false
                nextPair()
            } catch (t: Throwable) {
                voting = false
                _state.value = VoteState.Error(t.message ?: t.javaClass.simpleName)
            }
        }
    }

    fun undo() {
        val snap = undoStack.removeLastOrNull() ?: return
        viewModelScope.launch {
            try {
                Repo.undo(UndoRequest(
                    aId = snap.aId,
                    bId = snap.bId,
                    oldEloA = snap.oldEloA,
                    oldEloB = snap.oldEloB,
                    oldMatchesA = snap.oldMatchesA,
                    oldMatchesB = snap.oldMatchesB,
                ))
                pairer.forgetLast()
                val rows = Repo.rows.value
                val a = rows?.firstOrNull { it.id == snap.aId }
                val b = rows?.firstOrNull { it.id == snap.bId }
                if (a != null && b != null) {
                    currentPair = a to b
                }
                lastResult = null
                publish()
            } catch (t: Throwable) {
                undoStack.addLast(snap)
                _state.value = VoteState.Error(t.message ?: t.javaClass.simpleName)
            }
        }
    }

    private fun nextPair() {
        val rows = Repo.rows.value
        if (rows == null) {
            _state.value = VoteState.Loading
            return
        }
        val eligible = rows.eligibleIndices()
        if (eligible.size < 2) {
            currentPair = null
            _state.value = VoteState.NotEnough
            return
        }
        val pair = pairer.pick(rows, eligible)
        if (pair == null) {
            currentPair = null
            _state.value = VoteState.NoPair
            return
        }
        currentPair = pair
        publish()
    }

    private fun publish() {
        val pair = currentPair ?: return
        _state.value = VoteState.Ready(
            left = pair.first,
            right = pair.second,
            lastResult = lastResult,
            canUndo = undoStack.isNotEmpty(),
            voting = voting,
        )
    }
}
