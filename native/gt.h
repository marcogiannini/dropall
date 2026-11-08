/* Thin C API over libtorrent used by Dropall (see src/torrent.rs). */
#ifndef DROPALL_GT_H
#define DROPALL_GT_H
#include <stdint.h>
#ifdef __cplusplus
extern "C" {
#endif

typedef struct gt_session gt_session;

/* Torrent states reported in gt_info.state */
enum {
    GT_CHECKING = 0,
    GT_METADATA = 1,
    GT_DOWNLOADING = 2,
    GT_FINISHED = 3,
    GT_SEEDING = 4,
    GT_PAUSED = 5,
    GT_ERROR = 6
};

typedef struct gt_info {
    char hash[41];
    char name[256];
    char save_path[1024];
    char error[256];
    int32_t state;
    int32_t peers;
    int32_t seeds;
    int32_t has_metadata;
    int64_t total_wanted;
    int64_t total_done;
    int64_t down_rate;
    int64_t up_rate;
    int64_t uploaded;
} gt_info;

/* state_dir keeps resume data so torrents survive restarts. */
gt_session *gt_create(const char *state_dir, int listen_port, int test_mode);
void gt_destroy(gt_session *s);

/* Return 0 on success (hash_out = 40 hex chars + NUL), -1 on error (err filled). */
int gt_add_magnet(gt_session *s, const char *uri, const char *save_dir,
                  char *hash_out, char *err, int err_len);
int gt_add_file(gt_session *s, const char *path, const char *save_dir,
                char *hash_out, char *err, int err_len);

void gt_remove(gt_session *s, const char *hash, int delete_files);
void gt_pause(gt_session *s, const char *hash);
void gt_resume(gt_session *s, const char *hash);
/* kbps: 0 = unlimited. seed_after: keep seeding when download is complete. */
void gt_set_limits(gt_session *s, int down_kbps, int up_kbps);
void gt_set_seed_after(gt_session *s, int seed_after);

/* Fills up to max entries, returns the number of torrents. */
int gt_snapshot(gt_session *s, gt_info *out, int max);
/* Persists resume data for every torrent (blocks briefly). */
void gt_save_all(gt_session *s);

/* Test helpers: build a .torrent from a file and connect to a known peer. */
int gt_make_torrent(const char *content_path, const char *out_torrent, char *err, int err_len);
void gt_add_peer(gt_session *s, const char *hash, const char *ip, int port);
int gt_listen_port(gt_session *s);

const char *gt_version(void);

#ifdef __cplusplus
}
#endif
#endif
