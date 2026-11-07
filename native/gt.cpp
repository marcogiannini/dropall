// Thin C API over libtorrent used by Grabbit (see src/torrent.rs).
#include "gt.h"

#include <libtorrent/add_torrent_params.hpp>
#include <libtorrent/alert_types.hpp>
#include <libtorrent/bencode.hpp>
#include <libtorrent/create_torrent.hpp>
#include <libtorrent/error_code.hpp>
#include <libtorrent/hex.hpp>
#include <libtorrent/magnet_uri.hpp>
#include <libtorrent/read_resume_data.hpp>
#include <libtorrent/session.hpp>
#include <libtorrent/settings_pack.hpp>
#include <libtorrent/torrent_flags.hpp>
#include <libtorrent/torrent_handle.hpp>
#include <libtorrent/torrent_info.hpp>
#include <libtorrent/torrent_status.hpp>
#include <libtorrent/write_resume_data.hpp>

#include <chrono>
#include <cstdio>
#include <cstring>
#include <fstream>
#include <iterator>
#include <mutex>
#include <string>
#include <thread>
#include <vector>

#include <filesystem>

namespace lt = libtorrent;
namespace fs = std::filesystem;

struct gt_session {
    lt::session ses;
    std::string state_dir;
    std::mutex m;
    bool seed_after = false;
    int port = 6881;
    explicit gt_session(lt::settings_pack p) : ses(std::move(p)) {}
};

static void copy_str(char *dst, size_t cap, const std::string &s) {
    if (cap == 0) return;
    size_t n = s.size() < cap - 1 ? s.size() : cap - 1;
    std::memcpy(dst, s.data(), n);
    dst[n] = 0;
}

static std::string hash_of(lt::torrent_handle const &h) {
    auto ih = h.info_hashes();
    lt::sha1_hash v1 = ih.has_v1() ? ih.v1 : lt::sha1_hash(ih.v2.data());
    return lt::aux::to_hex(v1);
}

static lt::torrent_handle find(gt_session *s, const char *hash) {
    for (auto const &h : s->ses.get_torrents())
        if (h.is_valid() && hash_of(h) == hash) return h;
    return lt::torrent_handle();
}

static std::string resume_path(gt_session *s, std::string const &hash) {
    return (fs::path(s->state_dir) / (hash + ".resume")).string();
}

static void write_resume(gt_session *s, lt::add_torrent_params const &p, std::string const &hash) {
    auto buf = lt::write_resume_data_buf(p);
    std::string tmp = resume_path(s, hash) + ".tmp";
    {
        std::ofstream f(tmp, std::ios::binary | std::ios::trunc);
        f.write(buf.data(), (std::streamsize)buf.size());
    }
    std::error_code ec;
    fs::rename(tmp, resume_path(s, hash), ec);
}

// Drains the alert queue: stores resume data, pauses finished torrents when
// seeding is off.
static void pump(gt_session *s) {
    std::vector<lt::alert *> alerts;
    s->ses.pop_alerts(&alerts);
    for (lt::alert *a : alerts) {
        if (auto *r = lt::alert_cast<lt::save_resume_data_alert>(a)) {
            write_resume(s, r->params, hash_of(r->handle));
        } else if (auto *m = lt::alert_cast<lt::metadata_received_alert>(a)) {
            m->handle.save_resume_data(lt::torrent_handle::save_info_dict);
        } else if (auto *f = lt::alert_cast<lt::torrent_finished_alert>(a)) {
            f->handle.save_resume_data(lt::torrent_handle::save_info_dict);
            if (!s->seed_after) f->handle.pause();
        }
    }
}

static int add_params(gt_session *s, lt::add_torrent_params p, const char *save_dir,
                      char *hash_out, char *err, int err_len) {
    p.save_path = save_dir;
    lt::error_code ec;
    lt::torrent_handle h = s->ses.add_torrent(std::move(p), ec);
    if (ec) {
        copy_str(err, (size_t)err_len, ec.message());
        return -1;
    }
    std::string hash = hash_of(h);
    copy_str(hash_out, 41, hash);
    h.save_resume_data(lt::torrent_handle::save_info_dict);
    return 0;
}

extern "C" {

gt_session *gt_create(const char *state_dir, int listen_port, int test_mode) {
    lt::settings_pack p;
    p.set_str(lt::settings_pack::user_agent, "Grabbit");
    p.set_str(lt::settings_pack::peer_fingerprint, lt::generate_fingerprint("GB", 0, 1, 0, 0));
    if (listen_port <= 0) listen_port = 6881;
    {
        std::string port = std::to_string(listen_port);
        p.set_str(lt::settings_pack::listen_interfaces, "0.0.0.0:" + port + ",[::]:" + port);
    }
    bool net = test_mode == 0;
    p.set_bool(lt::settings_pack::enable_dht, net);
    p.set_bool(lt::settings_pack::enable_lsd, net);
    p.set_bool(lt::settings_pack::enable_upnp, net);
    p.set_bool(lt::settings_pack::enable_natpmp, net);
    p.set_str(lt::settings_pack::dht_bootstrap_nodes,
              "router.bittorrent.com:6881,router.utorrent.com:6881,dht.transmissionbt.com:6881,dht.libtorrent.org:25401");
    p.set_int(lt::settings_pack::alert_mask,
              lt::alert_category::error | lt::alert_category::status | lt::alert_category::storage);
    auto *s = new gt_session(std::move(p));
    s->state_dir = state_dir;
    s->port = listen_port;
    std::error_code ec;
    fs::create_directories(s->state_dir, ec);
    // Restore torrents saved by a previous run.
    for (auto const &e : fs::directory_iterator(s->state_dir, ec)) {
        if (e.path().extension() != ".resume") continue;
        std::ifstream f(e.path(), std::ios::binary);
        std::vector<char> buf((std::istreambuf_iterator<char>(f)), std::istreambuf_iterator<char>());
        lt::error_code rec;
        lt::add_torrent_params params = lt::read_resume_data(buf, rec);
        if (rec) continue;
        lt::error_code aec;
        s->ses.add_torrent(std::move(params), aec);
    }
    return s;
}

void gt_destroy(gt_session *s) {
    if (!s) return;
    gt_save_all(s);
    delete s;
}

int gt_add_magnet(gt_session *s, const char *uri, const char *save_dir, char *hash_out, char *err,
                  int err_len) {
    std::lock_guard<std::mutex> g(s->m);
    lt::error_code ec;
    lt::add_torrent_params p = lt::parse_magnet_uri(uri, ec);
    if (ec) {
        copy_str(err, (size_t)err_len, ec.message());
        return -1;
    }
    return add_params(s, std::move(p), save_dir, hash_out, err, err_len);
}

int gt_add_file(gt_session *s, const char *path, const char *save_dir, char *hash_out, char *err,
                int err_len) {
    std::lock_guard<std::mutex> g(s->m);
    lt::error_code ec;
    auto ti = std::make_shared<lt::torrent_info>(std::string(path), ec);
    if (ec) {
        copy_str(err, (size_t)err_len, ec.message());
        return -1;
    }
    lt::add_torrent_params p;
    p.ti = ti;
    return add_params(s, std::move(p), save_dir, hash_out, err, err_len);
}

void gt_remove(gt_session *s, const char *hash, int delete_files) {
    std::lock_guard<std::mutex> g(s->m);
    auto h = find(s, hash);
    if (h.is_valid())
        s->ses.remove_torrent(h, delete_files ? lt::session::delete_files : lt::remove_flags_t{});
    std::error_code ec;
    fs::remove(resume_path(s, hash), ec);
}

void gt_pause(gt_session *s, const char *hash) {
    std::lock_guard<std::mutex> g(s->m);
    auto h = find(s, hash);
    if (h.is_valid()) {
        h.unset_flags(lt::torrent_flags::auto_managed);
        h.pause();
        h.save_resume_data(lt::torrent_handle::save_info_dict);
    }
}

void gt_resume(gt_session *s, const char *hash) {
    std::lock_guard<std::mutex> g(s->m);
    auto h = find(s, hash);
    if (h.is_valid()) {
        h.set_flags(lt::torrent_flags::auto_managed);
        h.clear_error();
        h.resume();
    }
}

void gt_set_limits(gt_session *s, int down_kbps, int up_kbps) {
    std::lock_guard<std::mutex> g(s->m);
    lt::settings_pack p;
    p.set_int(lt::settings_pack::download_rate_limit, down_kbps > 0 ? down_kbps * 1024 : 0);
    p.set_int(lt::settings_pack::upload_rate_limit, up_kbps > 0 ? up_kbps * 1024 : 0);
    s->ses.apply_settings(p);
}

void gt_set_seed_after(gt_session *s, int seed_after) {
    std::lock_guard<std::mutex> g(s->m);
    s->seed_after = seed_after != 0;
}

int gt_snapshot(gt_session *s, gt_info *out, int max) {
    std::lock_guard<std::mutex> g(s->m);
    pump(s);
    int n = 0;
    for (auto const &h : s->ses.get_torrents()) {
        if (!h.is_valid()) continue;
        if (n >= max) break;
        lt::torrent_status st = h.status();
        gt_info &o = out[n++];
        std::memset(&o, 0, sizeof(o));
        copy_str(o.hash, sizeof(o.hash), hash_of(h));
        copy_str(o.name, sizeof(o.name), st.name);
        copy_str(o.save_path, sizeof(o.save_path), st.save_path);
        o.has_metadata = st.has_metadata ? 1 : 0;
        o.total_wanted = st.total_wanted;
        o.total_done = st.total_wanted_done;
        o.down_rate = st.download_payload_rate;
        o.up_rate = st.upload_payload_rate;
        o.uploaded = st.all_time_upload;
        o.peers = st.num_peers;
        o.seeds = st.num_seeds;
        bool paused = static_cast<bool>(st.flags & lt::torrent_flags::paused);
        if (st.errc) {
            o.state = GT_ERROR;
            copy_str(o.error, sizeof(o.error), st.errc.message());
        } else if (paused) {
            o.state = st.is_finished ? GT_FINISHED : GT_PAUSED;
        } else {
            switch (st.state) {
            case lt::torrent_status::checking_files:
            case lt::torrent_status::checking_resume_data:
                o.state = GT_CHECKING;
                break;
            case lt::torrent_status::downloading_metadata:
                o.state = GT_METADATA;
                break;
            case lt::torrent_status::finished:
                o.state = GT_FINISHED;
                break;
            case lt::torrent_status::seeding:
                o.state = GT_SEEDING;
                break;
            default:
                o.state = GT_DOWNLOADING;
            }
        }
    }
    return n;
}

void gt_save_all(gt_session *s) {
    int pending = 0;
    {
        std::lock_guard<std::mutex> g(s->m);
        pump(s); // store anything already queued
        for (auto const &h : s->ses.get_torrents()) {
            if (h.is_valid()) {
                h.save_resume_data(lt::torrent_handle::save_info_dict);
                ++pending;
            }
        }
    }
    auto end = std::chrono::steady_clock::now() + std::chrono::seconds(5);
    while (pending > 0 && std::chrono::steady_clock::now() < end) {
        s->ses.wait_for_alert(std::chrono::milliseconds(200));
        std::vector<lt::alert *> alerts;
        std::lock_guard<std::mutex> g(s->m);
        s->ses.pop_alerts(&alerts);
        for (lt::alert *a : alerts) {
            if (auto *r = lt::alert_cast<lt::save_resume_data_alert>(a)) {
                write_resume(s, r->params, hash_of(r->handle));
                --pending;
            } else if (lt::alert_cast<lt::save_resume_data_failed_alert>(a)) {
                --pending;
            }
        }
    }
}

int gt_make_torrent(const char *content_path, const char *out_torrent, char *err, int err_len) {
    try {
        lt::file_storage fsto;
        fs::path content = fs::absolute(content_path);
        lt::add_files(fsto, content.string());
        lt::create_torrent ct(fsto);
        lt::error_code ec;
        lt::set_piece_hashes(ct, content.parent_path().string(), ec);
        if (ec) {
            copy_str(err, (size_t)err_len, ec.message());
            return -1;
        }
        std::vector<char> buf;
        lt::bencode(std::back_inserter(buf), ct.generate());
        std::ofstream f(out_torrent, std::ios::binary | std::ios::trunc);
        f.write(buf.data(), (std::streamsize)buf.size());
        return 0;
    } catch (std::exception const &e) {
        copy_str(err, (size_t)err_len, e.what());
        return -1;
    }
}

void gt_add_peer(gt_session *s, const char *hash, const char *ip, int port) {
    std::lock_guard<std::mutex> g(s->m);
    auto h = find(s, hash);
    lt::error_code ec;
    auto addr = lt::make_address(ip, ec);
    if (h.is_valid() && !ec) h.connect_peer(lt::tcp::endpoint(addr, (unsigned short)port));
}

int gt_listen_port(gt_session *s) { return s->port; }

const char *gt_version(void) { return lt::version(); }

} // extern "C"
