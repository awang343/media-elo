package com.mediaelo.client.ui

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
import androidx.compose.material3.Button
import androidx.compose.material3.Card
import androidx.compose.material3.CardDefaults
import androidx.compose.material3.CircularProgressIndicator
import androidx.compose.material3.MaterialTheme
import androidx.compose.material3.OutlinedButton
import androidx.compose.material3.Text
import androidx.compose.runtime.Composable
import androidx.compose.runtime.collectAsState
import androidx.compose.runtime.getValue
import androidx.compose.ui.Alignment
import androidx.compose.ui.Modifier
import androidx.compose.ui.text.font.FontWeight
import androidx.compose.ui.text.style.TextAlign
import androidx.compose.ui.unit.dp
import androidx.lifecycle.viewmodel.compose.viewModel
import com.mediaelo.client.api.Row

@Composable
fun VoteScreen(
    contentPadding: PaddingValues,
    vm: VoteViewModel = viewModel(),
) {
    val state by vm.state.collectAsState()
    Box(modifier = Modifier.fillMaxSize().padding(contentPadding)) {
        when (val s = state) {
            VoteState.Loading -> CircularProgressIndicator(Modifier.align(Alignment.Center))
            is VoteState.Error -> CenteredMessage(
                title = "Could not reach server",
                detail = s.message,
                action = "Retry" to vm::retry,
            )
            VoteState.NotEnough -> CenteredMessage(
                title = "Not enough rankable rows",
                detail = "Mark at least two rows of the same type as done or dropped to start voting.",
                action = "Refresh" to vm::retry,
            )
            VoteState.NoPair -> CenteredMessage(
                title = "No new pair",
                detail = "Recent history covers everything available right now.",
                action = "Try again" to vm::skip,
            )
            is VoteState.Ready -> ReadyView(s, vm)
        }
    }
}

@Composable
private fun ReadyView(s: VoteState.Ready, vm: VoteViewModel) {
    Column(
        modifier = Modifier.fillMaxSize().padding(16.dp),
        verticalArrangement = Arrangement.spacedBy(12.dp),
    ) {
        Text(
            "Which is better?",
            style = MaterialTheme.typography.titleMedium,
            modifier = Modifier.fillMaxWidth(),
            textAlign = TextAlign.Center,
        )
        Text(
            s.left.type,
            style = MaterialTheme.typography.labelMedium,
            modifier = Modifier.fillMaxWidth(),
            textAlign = TextAlign.Center,
        )

        CandidateCard(
            row = s.left,
            enabled = !s.voting,
            modifier = Modifier.fillMaxWidth().weight(1f),
        ) { vm.vote(leftWins = true) }

        Text(
            "vs",
            style = MaterialTheme.typography.labelLarge,
            modifier = Modifier.fillMaxWidth(),
            textAlign = TextAlign.Center,
        )

        CandidateCard(
            row = s.right,
            enabled = !s.voting,
            modifier = Modifier.fillMaxWidth().weight(1f),
        ) { vm.vote(leftWins = false) }

        Row(
            modifier = Modifier.fillMaxWidth(),
            horizontalArrangement = Arrangement.spacedBy(8.dp),
        ) {
            OutlinedButton(
                onClick = vm::skip,
                enabled = !s.voting,
                modifier = Modifier.weight(1f),
            ) { Text("Skip") }
            OutlinedButton(
                onClick = vm::undo,
                enabled = !s.voting && s.canUndo,
                modifier = Modifier.weight(1f),
            ) { Text("Undo") }
        }

        s.lastResult?.let { ResultBanner(it) }
    }
}

@Composable
private fun CandidateCard(
    row: Row,
    enabled: Boolean,
    modifier: Modifier = Modifier,
    onClick: () -> Unit,
) {
    Card(
        modifier = modifier,
        onClick = onClick,
        enabled = enabled,
        colors = CardDefaults.cardColors(
            containerColor = MaterialTheme.colorScheme.primaryContainer,
            contentColor = MaterialTheme.colorScheme.onPrimaryContainer,
        ),
    ) {
        Column(
            modifier = Modifier.fillMaxSize().padding(16.dp),
            horizontalAlignment = Alignment.CenterHorizontally,
            verticalArrangement = Arrangement.Center,
        ) {
            Text(
                row.title,
                style = MaterialTheme.typography.titleLarge,
                fontWeight = FontWeight.SemiBold,
                textAlign = TextAlign.Center,
            )
            Spacer(Modifier.height(8.dp))
            Text(
                "elo ${"%.0f".format(row.elo)} • ${row.matches} matches",
                style = MaterialTheme.typography.bodyMedium,
            )
        }
    }
}

@Composable
private fun ResultBanner(result: VoteResult) {
    val w = if (result.deltaWinner >= 0) "+${"%.1f".format(result.deltaWinner)}"
    else "%.1f".format(result.deltaWinner)
    val l = if (result.deltaLoser >= 0) "+${"%.1f".format(result.deltaLoser)}"
    else "%.1f".format(result.deltaLoser)
    Text(
        "${result.winnerTitle} $w • ${result.loserTitle} $l",
        style = MaterialTheme.typography.bodySmall,
        modifier = Modifier.fillMaxWidth(),
        textAlign = TextAlign.Center,
    )
}

@Composable
private fun CenteredMessage(
    title: String,
    detail: String,
    action: Pair<String, () -> Unit>?,
) {
    Column(
        modifier = Modifier.fillMaxSize().padding(24.dp),
        horizontalAlignment = Alignment.CenterHorizontally,
        verticalArrangement = Arrangement.Center,
    ) {
        Text(title, style = MaterialTheme.typography.titleMedium)
        Spacer(Modifier.height(8.dp))
        Text(detail, style = MaterialTheme.typography.bodySmall, textAlign = TextAlign.Center)
        if (action != null) {
            Spacer(Modifier.height(16.dp))
            Button(onClick = action.second) { Text(action.first) }
        }
    }
}
