package com.mediaelo.client.api

import io.ktor.client.HttpClient
import io.ktor.client.call.body
import io.ktor.client.engine.okhttp.OkHttp
import io.ktor.client.plugins.contentnegotiation.ContentNegotiation
import io.ktor.client.request.delete
import io.ktor.client.request.get
import io.ktor.client.request.patch
import io.ktor.client.request.post
import io.ktor.client.request.put
import io.ktor.client.request.setBody
import io.ktor.http.ContentType
import io.ktor.http.contentType
import io.ktor.serialization.kotlinx.json.json
import kotlinx.serialization.json.Json

class MediaEloClient(private val baseUrl: String) {
    private val http = HttpClient(OkHttp) {
        install(ContentNegotiation) {
            json(Json {
                ignoreUnknownKeys = true
                explicitNulls = false
            })
        }
    }

    private fun url(path: String) = "${baseUrl.trimEnd('/')}$path"

    suspend fun listRows(): List<Row> =
        http.get(url("/rows")).body()

    suspend fun addRow(req: AddRequest): Row =
        http.post(url("/rows")) {
            contentType(ContentType.Application.Json)
            setBody(req)
        }.body()

    suspend fun editRow(id: String, req: EditRequest): Row =
        http.put(url("/rows/$id")) {
            contentType(ContentType.Application.Json)
            setBody(req)
        }.body()

    suspend fun deleteRow(id: String) {
        http.delete(url("/rows/$id"))
    }

    suspend fun setStatus(id: String, status: String): Row =
        http.patch(url("/rows/$id/status")) {
            contentType(ContentType.Application.Json)
            setBody(SetStatusRequest(status))
        }.body()

    suspend fun listTypes(): List<String> =
        http.get(url("/types")).body()

    suspend fun vote(winnerId: String, loserId: String): VoteResponse =
        http.post(url("/vote")) {
            contentType(ContentType.Application.Json)
            setBody(VoteRequest(winnerId, loserId))
        }.body()

    fun close() = http.close()
}
