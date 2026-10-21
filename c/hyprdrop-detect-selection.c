#define _GNU_SOURCE
#include <atspi/atspi.h>
#include <stdio.h>
#include <stdlib.h>
#include <string.h>
#include <unistd.h>
#include <dirent.h>
#include <sys/stat.h>
#include <ctype.h>

#define MAX_FILES 64
static char g_folder[2048] = {0};
static char g_files[MAX_FILES][1024];
static int g_file_count = 0;

static void add_file_if_exists(const char *dir, const char *name) {
    if (!dir || !dir[0] || !name || !name[0] || g_file_count >= MAX_FILES) return;

    char full[4096];
    size_t dlen = strlen(dir);
    if (dlen > 0 && dir[dlen - 1] == '/') {
        snprintf(full, sizeof(full), "%s%s", dir, name);
    } else {
        snprintf(full, sizeof(full), "%s/%s", dir, name);
    }

    if (access(full, F_OK) == 0) {
        // Avoid duplicates
        for (int i = 0; i < g_file_count; i++) {
            if (strcmp(g_files[i], full) == 0) return;
        }
        snprintf(g_files[g_file_count], sizeof(g_files[0]), "%s", full);
        g_file_count++;
    }
}

static void trim_quotes_and_spaces(char *str) {
    if (!str) return;
    char *start = str;
    while (*start == ' ' || *start == '\t' || *start == '\r' || *start == '\n' || *start == '"' || *start == '\'') {
        start++;
    }
    // Check unicode quotes at start: “ (\xe2\x80\x9c), ‘ (\xe2\x80\x98), „ (\xe2\x80\x9e), « (\xc2\xab)
    if ((unsigned char)start[0] == 0xe2 && (unsigned char)start[1] == 0x80 &&
        ((unsigned char)start[2] == 0x9c || (unsigned char)start[2] == 0x98 || (unsigned char)start[2] == 0x9e)) {
        start += 3;
    } else if ((unsigned char)start[0] == 0xc2 && (unsigned char)start[1] == 0xab) {
        start += 2;
    }

    size_t len = strlen(start);
    while (len > 0) {
        char end_char = start[len - 1];
        if (end_char == ' ' || end_char == '\t' || end_char == '\r' || end_char == '\n' || end_char == '"' || end_char == '\'') {
            start[--len] = '\0';
        } else if (len >= 3 && (unsigned char)start[len - 3] == 0xe2 && (unsigned char)start[len - 2] == 0x80 &&
                   ((unsigned char)start[len - 1] == 0x9d || (unsigned char)start[len - 1] == 0x99 || (unsigned char)start[len - 1] == 0x9c)) {
            len -= 3;
            start[len] = '\0';
        } else if (len >= 2 && (unsigned char)start[len - 2] == 0xc2 && (unsigned char)start[len - 1] == 0xbb) {
            len -= 2;
            start[len] = '\0';
        } else {
            break;
        }
    }
    if (start != str) {
        memmove(str, start, len + 1);
    }
}

static void parse_status_bar_text(const char *txt) {
    if (!txt || !txt[0]) return;

    // Strategy 1: Find text before pipe '|' (Thunar standard: [filename] | [size] | [type])
    const char *pipe = strstr(txt, "|");
    if (pipe) {
        char candidate[1024];
        size_t part_len = (size_t)(pipe - txt);
        if (part_len < sizeof(candidate)) {
            memcpy(candidate, txt, part_len);
            candidate[part_len] = '\0';
            trim_quotes_and_spaces(candidate);
            if (candidate[0]) {
                add_file_if_exists(g_folder, candidate);
                if (g_file_count > 0) return;
            }
        }
    }

    // Strategy 2: Quoted string anywhere in text
    // Check ASCII quotes
    const char *q1 = strchr(txt, '"');
    if (q1) {
        const char *q2 = strchr(q1 + 1, '"');
        if (q2 && (size_t)(q2 - q1 - 1) < sizeof(g_files[0])) {
            char candidate[1024];
            size_t n = (size_t)(q2 - q1 - 1);
            memcpy(candidate, q1 + 1, n);
            candidate[n] = '\0';
            add_file_if_exists(g_folder, candidate);
            if (g_file_count > 0) return;
        }
    }

    // Check Unicode quotes: “ (\xe2\x80\x9c) ... ” (\xe2\x80\x9d)
    const char *uq1 = strstr(txt, "\xe2\x80\x9c");
    if (uq1) {
        const char *uq2 = strstr(uq1 + 3, "\xe2\x80\x9d");
        if (uq2 && (size_t)(uq2 - uq1 - 3) < sizeof(g_files[0])) {
            char candidate[1024];
            size_t n = (size_t)(uq2 - uq1 - 3);
            memcpy(candidate, uq1 + 3, n);
            candidate[n] = '\0';
            add_file_if_exists(g_folder, candidate);
            if (g_file_count > 0) return;
        }
    }
}

static void walk(AtspiAccessible *obj, int depth) {
    if (depth > 15 || !obj) return;
    int count = atspi_accessible_get_child_count(obj, NULL);
    for (int i = 0; i < count; i++) {
        AtspiAccessible *c = atspi_accessible_get_child_at_index(obj, i, NULL);
        if (!c) continue;
        char *role = atspi_accessible_get_role_name(c, NULL);
        if (role) {
            // Folder location bar detection
            if (!g_folder[0] && (strcmp(role, "text") == 0 || strcmp(role, "entry") == 0)) {
                AtspiText *t = atspi_accessible_get_text_iface(c);
                if (t) {
                    char *txt = atspi_text_get_text(t, 0, -1, NULL);
                    if (txt) {
                        char *p = txt;
                        while (*p == ' ' || *p == '\t' || *p == '\n') p++;
                        if (*p == '/') {
                            strncpy(g_folder, p, sizeof(g_folder) - 1);
                            size_t flen = strlen(g_folder);
                            while (flen > 0 && (g_folder[flen - 1] == ' ' || g_folder[flen - 1] == '\n' || g_folder[flen - 1] == '\r')) {
                                g_folder[--flen] = '\0';
                            }
                        }
                        g_free(txt);
                    }
                }
            }
            // Status bar detection
            else if (strcmp(role, "status bar") == 0) {
                char *txt = atspi_accessible_get_name(c, NULL);
                if (txt) {
                    parse_status_bar_text(txt);
                    g_free(txt);
                }
            }
            // State selected check for list/grid items
            else if (strcmp(role, "table cell") == 0 || strcmp(role, "list item") == 0 ||
                     strcmp(role, "icon") == 0 || strcmp(role, "table row") == 0) {
                AtspiStateSet *ss = atspi_accessible_get_state_set(c);
                if (ss) {
                    if (atspi_state_set_contains(ss, ATSPI_STATE_SELECTED)) {
                        char *name = atspi_accessible_get_name(c, NULL);
                        if (name && name[0] && g_folder[0]) {
                            add_file_if_exists(g_folder, name);
                        }
                        if (name) g_free(name);
                    }
                    g_object_unref(ss);
                }
            }
            g_free(role);
        }
        walk(c, depth + 1);
        g_object_unref(c);
    }
}

static gboolean is_accessible_active(AtspiAccessible *obj) {
    if (!obj) return FALSE;
    AtspiStateSet *ss = atspi_accessible_get_state_set(obj);
    if (!ss) return FALSE;
    gboolean active = atspi_state_set_contains(ss, ATSPI_STATE_ACTIVE) ||
                      atspi_state_set_contains(ss, ATSPI_STATE_FOCUSED);
    g_object_unref(ss);
    return active;
}

int main(int argc, char **argv) {
    (void)argc;
    (void)argv;

    if (atspi_init() != 0) return 1;
    AtspiAccessible *desktop = atspi_get_desktop(0);
    if (!desktop) return 1;

    int count = atspi_accessible_get_child_count(desktop, NULL);

    // Pass 1: Try active/focused windows first
    for (int i = 0; i < count; i++) {
        AtspiAccessible *app = atspi_accessible_get_child_at_index(desktop, i, NULL);
        if (!app) continue;
        char *name = atspi_accessible_get_name(app, NULL);
        if (name && (strcasestr(name, "thunar") || strcasestr(name, "nautilus") ||
                     strcasestr(name, "nemo") || strcasestr(name, "caja") ||
                     strcasestr(name, "dolphin") || strcasestr(name, "pcmanfm"))) {
            int wcount = atspi_accessible_get_child_count(app, NULL);
            for (int j = 0; j < wcount; j++) {
                AtspiAccessible *win = atspi_accessible_get_child_at_index(app, j, NULL);
                if (win) {
                    if (is_accessible_active(win)) {
                        walk(win, 0);
                    }
                    g_object_unref(win);
                    if (g_file_count > 0) break;
                }
            }
        }
        if (name) g_free(name);
        g_object_unref(app);
        if (g_file_count > 0) break;
    }

    // Pass 2: If not found in active window, inspect all file manager windows
    if (g_file_count == 0) {
        for (int i = 0; i < count; i++) {
            AtspiAccessible *app = atspi_accessible_get_child_at_index(desktop, i, NULL);
            if (!app) continue;
            char *name = atspi_accessible_get_name(app, NULL);
            if (name && (strcasestr(name, "thunar") || strcasestr(name, "nautilus") ||
                         strcasestr(name, "nemo") || strcasestr(name, "caja") ||
                         strcasestr(name, "dolphin") || strcasestr(name, "pcmanfm"))) {
                int wcount = atspi_accessible_get_child_count(app, NULL);
                for (int j = 0; j < wcount; j++) {
                    AtspiAccessible *win = atspi_accessible_get_child_at_index(app, j, NULL);
                    if (win) {
                        walk(win, 0);
                        g_object_unref(win);
                        if (g_file_count > 0) break;
                    }
                }
            }
            if (name) g_free(name);
            g_object_unref(app);
            if (g_file_count > 0) break;
        }
    }

    g_object_unref(desktop);

    // Fallback: If status bar indicated a single file and we have folder but no filename
    if (g_file_count == 0 && g_folder[0]) {
        DIR *d = opendir(g_folder);
        if (d) {
            struct dirent *de;
            char single_file[1024] = {0};
            int count_regular = 0;
            while ((de = readdir(d)) != NULL) {
                if (de->d_name[0] == '.') continue;
                count_regular++;
                if (count_regular == 1) {
                    snprintf(single_file, sizeof(single_file), "%s", de->d_name);
                }
            }
            closedir(d);
            if (count_regular == 1 && single_file[0]) {
                add_file_if_exists(g_folder, single_file);
            }
        }
    }

    if (g_file_count > 0) {
        for (int i = 0; i < g_file_count; i++) {
            printf("%s\n", g_files[i]);
        }
        return 0;
    }
    return 1;
}
