package com.mediaelo.client.ui

import androidx.compose.material.icons.Icons
import androidx.compose.material.icons.automirrored.outlined.List
import androidx.compose.material.icons.outlined.ThumbUp
import androidx.compose.material3.ExperimentalMaterial3Api
import androidx.compose.material3.Icon
import androidx.compose.material3.NavigationBar
import androidx.compose.material3.NavigationBarItem
import androidx.compose.material3.Scaffold
import androidx.compose.material3.Text
import androidx.compose.material3.TopAppBar
import androidx.compose.runtime.Composable
import androidx.compose.runtime.getValue
import androidx.compose.runtime.mutableStateOf
import androidx.compose.runtime.saveable.rememberSaveable
import androidx.compose.runtime.setValue
import androidx.compose.ui.graphics.vector.ImageVector

private enum class Tab(val label: String, val icon: ImageVector) {
    Library("Library", Icons.AutoMirrored.Outlined.List),
    Vote("Vote", Icons.Outlined.ThumbUp),
}

@OptIn(ExperimentalMaterial3Api::class)
@Composable
fun AppRoot() {
    var selected by rememberSaveable { mutableStateOf(Tab.Library) }
    Scaffold(
        topBar = { TopAppBar(title = { Text("Media Elo") }) },
        bottomBar = {
            NavigationBar {
                Tab.entries.forEach { tab ->
                    NavigationBarItem(
                        selected = selected == tab,
                        onClick = { selected = tab },
                        icon = { Icon(tab.icon, contentDescription = tab.label) },
                        label = { Text(tab.label) },
                    )
                }
            }
        },
    ) { padding ->
        when (selected) {
            Tab.Library -> LibraryScreen(contentPadding = padding)
            Tab.Vote -> VoteScreen(contentPadding = padding)
        }
    }
}
