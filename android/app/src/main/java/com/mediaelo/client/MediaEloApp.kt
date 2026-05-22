package com.mediaelo.client

import android.app.Application
import com.mediaelo.client.data.Settings

class MediaEloApp : Application() {
    override fun onCreate() {
        super.onCreate()
        Settings.init(this)
    }
}
