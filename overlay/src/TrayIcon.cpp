#include "../include/TrayIcon.hpp"
#include "../include/Rpc.hpp"

#include <gio/gio.h>
#include <unistd.h>

#include <cstdio>
#include <cstring>

namespace gsr {
namespace {

constexpr const char *sni_xml = R"xml(
<node>
  <interface name="org.freedesktop.DBus.Properties">
    <method name="Get"><arg type="s" direction="in"/><arg type="s" direction="in"/><arg type="v" direction="out"/></method>
    <method name="GetAll"><arg type="s" direction="in"/><arg type="a{sv}" direction="out"/></method>
  </interface>
  <interface name="org.kde.StatusNotifierItem">
    <method name="Activate"><arg type="i" direction="in"/><arg type="i" direction="in"/></method>
    <method name="SecondaryActivate"><arg type="i" direction="in"/><arg type="i" direction="in"/></method>
    <method name="ContextMenu"><arg type="i" direction="in"/><arg type="i" direction="in"/></method>
    <method name="Scroll"><arg type="i" direction="in"/><arg type="s" direction="in"/></method>
  </interface>
  <interface name="com.canonical.dbusmenu">
    <method name="GetLayout"><arg type="i" direction="in"/><arg type="u" direction="in"/><arg type="as" direction="in"/><arg type="u" direction="out"/><arg type="(ia{sv}av)" direction="out"/></method>
    <method name="Event"><arg type="i" direction="in"/><arg type="s" direction="in"/><arg type="v" direction="in"/><arg type="u" direction="in"/></method>
  </interface>
</node>
)xml";

struct TrayContext {
    GDBusConnection *connection = nullptr;
    std::string service_name;
    Rpc rpc_client;
    bool rpc_ready = false;
    guint properties_registration = 0;
    guint item_registration = 0;
    guint menu_registration = 0;
};

void send_cmd(TrayContext *ctx, const char *cmd) {
    if(!ctx->rpc_ready) {
        if(ctx->rpc_client.open("sgsr-ui") != RpcOpenResult::OK)
            return;
        ctx->rpc_ready = true;
    }

    char buf[64];
    snprintf(buf, sizeof(buf), "%s\n", cmd);
    if(!ctx->rpc_client.write(buf, strlen(buf)))
        ctx->rpc_ready = false;
}

GVariant *property_value(const char *property) {
    if(strcmp(property, "Category") == 0)
        return g_variant_new_string("ApplicationStatus");
    if(strcmp(property, "Id") == 0)
        return g_variant_new_string("sourisgsr");
    if(strcmp(property, "Title") == 0)
        return g_variant_new_string("SourisGSR");
    if(strcmp(property, "Status") == 0)
        return g_variant_new_string("Active");
    if(strcmp(property, "IconName") == 0)
        return g_variant_new_string("sourisgsr");
    if(strcmp(property, "Menu") == 0)
        return g_variant_new_object_path("/Menu");
    return nullptr;
}

void add_menu_item(GVariantBuilder *children, int id, const char *label) {
    GVariantBuilder properties;
    g_variant_builder_init(&properties, G_VARIANT_TYPE("a{sv}"));
    g_variant_builder_add(&properties, "{sv}", "label", g_variant_new_string(label));
    g_variant_builder_add(&properties, "{sv}", "enabled", g_variant_new_boolean(true));
    g_variant_builder_add(&properties, "{sv}", "visible", g_variant_new_boolean(true));

    GVariantBuilder submenu;
    g_variant_builder_init(&submenu, G_VARIANT_TYPE("av"));
    GVariant *item = g_variant_new("(i@a{sv}@av)", id,
        g_variant_builder_end(&properties), g_variant_builder_end(&submenu));
    g_variant_builder_add(children, "v", item);
}

GVariant *menu_layout() {
    GVariantBuilder properties;
    g_variant_builder_init(&properties, G_VARIANT_TYPE("a{sv}"));

    GVariantBuilder children;
    g_variant_builder_init(&children, G_VARIANT_TYPE("av"));
    add_menu_item(&children, 1, "Mostrar");
    add_menu_item(&children, 2, "Grabar");
    add_menu_item(&children, 3, "Replay");
    add_menu_item(&children, 4, "Guardar replay");
    add_menu_item(&children, 5, "Salir");

    GVariant *root = g_variant_new("(i@a{sv}@av)", 0,
        g_variant_builder_end(&properties), g_variant_builder_end(&children));
    return g_variant_new("(u@(ia{sv}av))", 1, root);
}

void method_call(GDBusConnection *, const gchar *, const gchar *,
                 const gchar *interface_name, const gchar *method_name,
                 GVariant *parameters,
                 GDBusMethodInvocation *invocation, gpointer user_data) {
    TrayContext *ctx = static_cast<TrayContext *>(user_data);

    if(strcmp(interface_name, "org.freedesktop.DBus.Properties") == 0) {
        if(strcmp(method_name, "Get") == 0) {
            const char *interface = nullptr;
            const char *property = nullptr;
            g_variant_get(parameters, "(&s&s)", &interface, &property);
            if(strcmp(interface, "org.kde.StatusNotifierItem") != 0)
                g_dbus_method_invocation_return_dbus_error(invocation, "com.sourisgsr.Error", "Unknown interface");
            else if(GVariant *value = property_value(property))
                g_dbus_method_invocation_return_value(invocation, g_variant_new("(@v)", g_variant_new_variant(value)));
            else
                g_dbus_method_invocation_return_dbus_error(invocation, "com.sourisgsr.Error", "Unknown property");
            return;
        }

        const char *interface = nullptr;
        g_variant_get(parameters, "(&s)", &interface);
        if(strcmp(method_name, "GetAll") == 0 && strcmp(interface, "org.kde.StatusNotifierItem") == 0) {
            GVariantBuilder properties;
            g_variant_builder_init(&properties, G_VARIANT_TYPE("a{sv}"));
            // NOTE: "{sv}" already wraps the value into a variant, so pass the
            // plain GVariant here — wrapping it again yields "variant variant
            // string", which Plasma rejects (item never shows in the tray).
            for(const char *name : {"Category", "Id", "Title", "Status", "IconName", "Menu"})
                g_variant_builder_add(&properties, "{sv}", name, property_value(name));
            g_dbus_method_invocation_return_value(invocation, g_variant_new("(@a{sv})", g_variant_builder_end(&properties)));
            return;
        }
        g_dbus_method_invocation_return_dbus_error(invocation, "com.sourisgsr.Error", "Unknown property request");
        return;
    }

    if(strcmp(interface_name, "com.canonical.dbusmenu") == 0) {
        if(strcmp(method_name, "GetLayout") == 0) {
            g_dbus_method_invocation_return_value(invocation, menu_layout());
            return;
        }
        if(strcmp(method_name, "Event") == 0) {
            int id = 0;
            const char *event = nullptr;
            GVariant *data = nullptr;
            guint timestamp = 0;
            g_variant_get(parameters, "(i&svu)", &id, &event, &data, &timestamp);
            if(data)
                g_variant_unref(data);
            if(strcmp(event, "clicked") == 0) {
                if(id == 1) send_cmd(ctx, "toggle-show");
                else if(id == 2) send_cmd(ctx, "toggle-record");
                else if(id == 3) send_cmd(ctx, "toggle-replay");
                else if(id == 4) send_cmd(ctx, "replay-save");
                else if(id == 5) send_cmd(ctx, "quit");
            }
            g_dbus_method_invocation_return_value(invocation, nullptr);
            return;
        }
    }

    if(strcmp(interface_name, "org.kde.StatusNotifierItem") == 0) {
        if(strcmp(method_name, "Activate") == 0 || strcmp(method_name, "SecondaryActivate") == 0) {
            send_cmd(ctx, "toggle-show");
            g_dbus_method_invocation_return_value(invocation, nullptr);
            return;
        }
        if(strcmp(method_name, "ContextMenu") == 0) {
            g_dbus_method_invocation_return_value(invocation, nullptr);
            return;
        }
        if(strcmp(method_name, "Scroll") == 0) {
            g_dbus_method_invocation_return_value(invocation, nullptr);
            return;
        }
    }

    g_dbus_method_invocation_return_dbus_error(invocation, "com.sourisgsr.Error", "Unknown method");
}

const GDBusInterfaceVTable vtable = {method_call, nullptr, nullptr};

bool register_item(TrayContext *ctx) {
    GError *error = nullptr;
    GDBusNodeInfo *node = g_dbus_node_info_new_for_xml(sni_xml, &error);
    if(!node) {
        fprintf(stderr, "Error: failed to describe tray menu: %s\n", error->message);
        g_error_free(error);
        return false;
    }

    GDBusInterfaceInfo *sni = g_dbus_node_info_lookup_interface(node, "org.freedesktop.DBus.Properties");
    GDBusInterfaceInfo *item = g_dbus_node_info_lookup_interface(node, "org.kde.StatusNotifierItem");
    GDBusInterfaceInfo *menu = g_dbus_node_info_lookup_interface(node, "com.canonical.dbusmenu");
    ctx->properties_registration = g_dbus_connection_register_object(ctx->connection, "/StatusNotifierItem", sni, &vtable, ctx, nullptr, &error);
    if(ctx->properties_registration)
        ctx->item_registration = g_dbus_connection_register_object(ctx->connection, "/StatusNotifierItem", item, &vtable, ctx, nullptr, &error);
    if(ctx->item_registration)
        ctx->menu_registration = g_dbus_connection_register_object(ctx->connection, "/Menu", menu, &vtable, ctx, nullptr, &error);
    g_dbus_node_info_unref(node);
    if(error) {
        fprintf(stderr, "Error: failed to register tray item: %s\n", error->message);
        g_error_free(error);
    }
    return ctx->properties_registration != 0 && ctx->item_registration != 0 && ctx->menu_registration != 0;
}

} // namespace

TrayIcon::TrayIcon() = default;
TrayIcon::~TrayIcon() { stop(); }

bool TrayIcon::start() {
    if(started.exchange(true))
        return running.load();
    running.store(true);
    settled.store(false);
    worker = std::thread(&TrayIcon::thread_main, this);
    for(int i = 0; i < 50 && !settled.load(); ++i)
        usleep(20 * 1000);
    return running.load();
}

void TrayIcon::stop() {
    running.store(false);
    if(worker.joinable())
        worker.join();
    started.store(false);
}

bool TrayIcon::is_running() const { return running.load(); }

void TrayIcon::thread_main() {
    GError *error = nullptr;
    GDBusConnection *connection = g_bus_get_sync(G_BUS_TYPE_SESSION, nullptr, &error);
    if(!connection) {
        if(error) g_error_free(error);
        settled.store(true);
        running.store(false);
        return;
    }

    TrayContext ctx;
    ctx.connection = connection;
    char name[128];
    snprintf(name, sizeof(name), "org.kde.StatusNotifierItem-%d", (int)getpid());
    ctx.service_name = name;

    GVariant *reply = g_dbus_connection_call_sync(connection, "org.freedesktop.DBus", "/org/freedesktop/DBus",
        "org.freedesktop.DBus", "RequestName", g_variant_new("(su)", name, 4),
        G_VARIANT_TYPE("(u)"), G_DBUS_CALL_FLAGS_NONE, 2000, nullptr, &error);
    if(!reply || error || !register_item(&ctx)) {
        if(error) g_error_free(error);
        if(reply) g_variant_unref(reply);
        g_object_unref(connection);
        settled.store(true);
        running.store(false);
        return;
    }
    g_variant_unref(reply);

    const char *registration_name = g_dbus_connection_get_unique_name(connection);
    g_dbus_connection_call_sync(connection, "org.kde.StatusNotifierWatcher", "/StatusNotifierWatcher",
        "org.kde.StatusNotifierWatcher", "RegisterStatusNotifierItem",
        g_variant_new("(s)", registration_name), nullptr, G_DBUS_CALL_FLAGS_NONE, 2000, nullptr, nullptr);
    settled.store(true);
    int watcher_refresh = 0;
    while(running.load())
    {
        while(g_main_context_pending(nullptr))
            g_main_context_iteration(nullptr, false);
        if(++watcher_refresh >= 1000) {
            watcher_refresh = 0;
            g_dbus_connection_call_sync(connection, "org.kde.StatusNotifierWatcher", "/StatusNotifierWatcher",
                "org.kde.StatusNotifierWatcher", "RegisterStatusNotifierItem",
                g_variant_new("(s)", registration_name), nullptr, G_DBUS_CALL_FLAGS_NONE, 2000, nullptr, nullptr);
        }
        g_usleep(10000);
    }

    if(ctx.properties_registration)
        g_dbus_connection_unregister_object(connection, ctx.properties_registration);
    if(ctx.item_registration)
        g_dbus_connection_unregister_object(connection, ctx.item_registration);
    if(ctx.menu_registration)
        g_dbus_connection_unregister_object(connection, ctx.menu_registration);
    g_object_unref(connection);
}

} // namespace gsr
