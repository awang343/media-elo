package com.mediaelo.client.data

import com.mediaelo.client.api.Row
import kotlin.math.abs
import kotlin.random.Random

/**
 * Port of rust/crates/client/src/pairer.rs.
 *
 * Strategy:
 *  - Pick a candidate A weighted by 1/(1+matches) so under-rated rows surface.
 *  - Pick B from same type, preferring unmatched first, then weighted by
 *    1/(1+matches) × 1/(1+|elo_a - elo_b|) with a 20% pure-random fallback.
 *  - Avoid the last 30 pairs by ID to keep things fresh.
 */
class Pairer(
    private val historySize: Int = 30,
    private val pairAttempts: Int = 20,
    private val randomPairChance: Double = 0.2,
    private val rng: Random = Random.Default,
) {
    private val recent: ArrayDeque<Pair<String, String>> = ArrayDeque(historySize)

    private fun pairId(a: Row, b: Row): Pair<String, String> =
        if (a.id <= b.id) a.id to b.id else b.id to a.id

    fun pick(rows: List<Row>, eligibleIdx: List<Int>): Pair<Row, Row>? {
        if (eligibleIdx.size < 2) return null

        val aIdx = pickCandidateIdx(rows, eligibleIdx)
        val aType = rows[aIdx].type

        val sameType = eligibleIdx.filter { it != aIdx && rows[it].type == aType }
        if (sameType.isEmpty()) return null

        val unmatched = sameType.filter { rows[it].matches == 0 }
        if (unmatched.isNotEmpty()) {
            repeat(pairAttempts) {
                val bIdx = unmatched.random(rng)
                val pid = pairId(rows[aIdx], rows[bIdx])
                if (pid !in recent) return rows[aIdx] to rows[bIdx]
            }
        }

        repeat(pairAttempts) {
            val bIdx = if (rng.nextDouble() < randomPairChance) {
                sameType.random(rng)
            } else {
                weightedOpponentIdx(rows, aIdx, sameType)
            }
            val pid = pairId(rows[aIdx], rows[bIdx])
            if (pid !in recent) return rows[aIdx] to rows[bIdx]
        }
        return null
    }

    fun remember(a: Row, b: Row) {
        if (recent.size == historySize) recent.removeFirst()
        recent.addLast(pairId(a, b))
    }

    fun forgetLast() { recent.removeLastOrNull() }

    private fun pickCandidateIdx(rows: List<Row>, pool: List<Int>): Int {
        val weights = pool.map { 1.0 / (1.0 + rows[it].matches) }
        return weightedPickIdx(pool, weights)
    }

    private fun weightedOpponentIdx(rows: List<Row>, aIdx: Int, candidates: List<Int>): Int {
        val aElo = rows[aIdx].elo
        val weights = candidates.map { i ->
            val b = rows[i]
            (1.0 / (1.0 + b.matches)) * (1.0 / (1.0 + abs(aElo - b.elo)))
        }
        return weightedPickIdx(candidates, weights)
    }

    private fun weightedPickIdx(pool: List<Int>, weights: List<Double>): Int {
        val total = weights.sum()
        if (total <= 0.0) return pool.first()
        var t = rng.nextDouble() * total
        for (i in weights.indices) {
            t -= weights[i]
            if (t <= 0.0) return pool[i]
        }
        return pool.last()
    }
}

private val RANKABLE = setOf("done", "dropped")

fun List<Row>.eligibleIndices(typeFilter: String? = null): List<Int> =
    withIndex()
        .filter { (_, r) -> r.status in RANKABLE && (typeFilter == null || r.type == typeFilter) }
        .map { it.index }
