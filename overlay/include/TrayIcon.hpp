#pragma once

#include <atomic>
#include <string>
#include <thread>

// SourisGSR: system tray icon, integrated in sgsr-ui only (never in the SDK).
// Speaks org.kde.StatusNotifierItem over raw libdbus (same style as the
// desktop-portal client in core/src/dbus.c, no new dependencies).
// User actions are delivered as Rpc command names ("toggle-show", ...),
// which the main loop executes on the UI thread — the D-Bus thread never
// touches Overlay state directly.
namespace gsr {
    class TrayIcon {
    public:
        TrayIcon();
        TrayIcon(const TrayIcon&) = delete;
        TrayIcon& operator=(const TrayIcon&) = delete;
        ~TrayIcon();

        // Registers the item and starts the dispatch thread.
        // Returns false (silent, no watcher/bus) when tray is unavailable.
        bool start();
        void stop();
        bool is_running() const;
    private:
        void thread_main();
    private:
        std::thread worker;
        std::atomic<bool> running{false};
        std::atomic<bool> started{false};
        std::atomic<bool> settled{false};
    };
}
