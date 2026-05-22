package com.mediaelo.client.api

import kotlinx.serialization.SerialName
import kotlinx.serialization.Serializable

@Serializable
data class Row(
    val id: String,
    @SerialName("type") val type: String,
    val title: String,
    val elo: Double,
    val matches: Int,
    val status: String,
    @SerialName("date_added") val dateAdded: String = "",
)

@Serializable
data class AddRequest(
    @SerialName("type") val type: String,
    val title: String,
    val rating: Double? = null,
    val status: String,
)

@Serializable
data class EditRequest(
    @SerialName("type") val type: String,
    val title: String,
    val status: String,
)

@Serializable
data class SetStatusRequest(val status: String)

@Serializable
data class VoteRequest(
    @SerialName("winner_id") val winnerId: String,
    @SerialName("loser_id") val loserId: String,
)

@Serializable
data class VoteResponse(
    val winner: Row,
    val loser: Row,
    @SerialName("delta_winner") val deltaWinner: Double,
    @SerialName("delta_loser") val deltaLoser: Double,
)

@Serializable
data class AddTypeRequest(val name: String)

@Serializable
data class RenameTypeRequest(@SerialName("new_name") val newName: String)

@Serializable
data class ReorderTypesRequest(val names: List<String>)

@Serializable
data class UndoRequest(
    @SerialName("a_id") val aId: String,
    @SerialName("b_id") val bId: String,
    @SerialName("old_elo_a") val oldEloA: Double,
    @SerialName("old_elo_b") val oldEloB: Double,
    @SerialName("old_matches_a") val oldMatchesA: Int,
    @SerialName("old_matches_b") val oldMatchesB: Int,
)
