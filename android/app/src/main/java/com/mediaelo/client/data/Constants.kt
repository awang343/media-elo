package com.mediaelo.client.data

/** Mirror of media_elo_core::STATUSES. Order matches the server's defaults. */
val STATUSES: List<String> = listOf(
    "backlog",
    "in progress",
    "on hold",
    "done",
    "dropped",
)

const val STATUS_BACKLOG = "backlog"
const val STATUS_DONE = "done"
const val STATUS_DROPPED = "dropped"
