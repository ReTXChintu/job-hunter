import 'dart:async';
import 'dart:convert';

import 'package:flutter/foundation.dart';
import 'package:shared_preferences/shared_preferences.dart';

import '../logic/labels.dart';
import '../models/application.dart';
import '../models/dashboard.dart';
import '../models/envelope.dart';
import 'connection_controller.dart';
import 'data_source.dart';

/// The applications list/detail data and the actions a phone may take on
/// them. Every action is one `connection.request(...)` call that reaches the
/// exact same desktop-side `orchestrator` function the Tauri UI calls (see
/// `crates/job-hunter-core/src/remote/dispatch.rs`) -- this class never
/// decides an application's fate on its own, it only asks the desktop to.
/// Reads come from the desktop when it's online, else from what it last
/// synced to the server.
class ApplicationsController extends ChangeNotifier {
  static const _cacheKey = 'job_hunter.applications_cache.v2';

  final ConnectionController connection;
  StreamSubscription? _changedSub;
  StreamSubscription? _resyncSub;

  ApplicationsController(this.connection) {
    _changedSub = connection.changed.listen((update) {
      if (update.collections.any((c) => const {'applications', 'jobs', 'job_analyses', 'agent_runs', 'resumes', 'cover_letters'}.contains(c))) {
        refresh();
      }
    });
    _resyncSub = connection.resync.listen((_) => refresh());
    _loadCache();
  }

  List<ApplicationListItem> items = [];
  Dashboard? dashboard;
  bool loading = false;
  DataSource source = DataSource.none;
  String? error;
  DateTime? lastLoadedAt;
  bool _refreshQueued = false;
  Future<void>? _inFlight;

  bool get isFromCache => source == DataSource.cache;
  bool get isFromServer => source == DataSource.server;

  Future<void> _loadCache() async {
    try {
      final prefs = await SharedPreferences.getInstance();
      final raw = prefs.getString(_cacheKey);
      if (raw == null || source != DataSource.none) return;
      items = parseApplicationList(jsonDecode(raw));
      source = DataSource.cache;
      notifyListeners();
    } catch (_) {
      // Corrupt cache: ignore, a live refresh will replace it.
    }
  }

  Future<void> _saveCache(dynamic raw) async {
    try {
      final prefs = await SharedPreferences.getInstance();
      await prefs.setString(_cacheKey, jsonEncode(raw));
    } catch (_) {}
  }

  /// Reloads the list (and the desktop's dashboard counts). Calls made while
  /// one is running coalesce into one more run afterwards.
  Future<void> refresh() {
    if (_inFlight != null) {
      _refreshQueued = true;
      return _inFlight!;
    }
    final run = _refresh().whenComplete(() {
      _inFlight = null;
      if (_refreshQueued) {
        _refreshQueued = false;
        refresh();
      }
    });
    _inFlight = run;
    return run;
  }

  Future<void> _refresh() async {
    loading = true;
    notifyListeners();
    final resp = await connection.request('list_applications');
    dynamic data = resp.ok ? resp.data : null;
    var from = DataSource.desktop;
    if (data is! List) {
      // With the desktop offline, read what it last synced to the server.
      data = await connection.serverGet('/v1/applications');
      from = DataSource.server;
    }
    if (data is List) {
      items = parseApplicationList(data);
      source = from;
      lastLoadedAt = DateTime.now();
      error = null;
      unawaited(_saveCache(data));
    } else {
      error = resp.ok ? 'Unexpected reply from the desktop.' : friendlyError(resp);
    }
    if (from == DataSource.desktop) {
      final dashResp = await connection.request('list_dashboard');
      if (dashResp.ok && dashResp.data is Map) {
        dashboard = Dashboard.fromJson((dashResp.data as Map).cast<String, dynamic>());
      }
    }
    loading = false;
    notifyListeners();
  }

  /// The full application, from the desktop or else the server (which has
  /// no resume/cover letter). Returns null with [lastDetailError] set.
  String? lastDetailError;
  bool lastDetailFromServer = false;

  Future<ApplicationDetail?> loadDetail(String applicationId) async {
    final resp = await connection.request('get_application', {'id': applicationId});
    if (resp.ok && resp.data is Map) {
      lastDetailFromServer = false;
      lastDetailError = null;
      return ApplicationDetail.fromJson((resp.data as Map).cast<String, dynamic>());
    }
    final data = await connection.serverGet('/v1/applications/${Uri.encodeComponent(applicationId)}');
    if (data is Map && data['application'] is Map) {
      lastDetailFromServer = true;
      lastDetailError = null;
      return ApplicationDetail.fromJson(data.cast<String, dynamic>());
    }
    lastDetailError = resp.ok ? 'Could not load this application.' : friendlyError(resp);
    error = lastDetailError;
    notifyListeners();
    return null;
  }

  Future<RelayResponse> approve(String applicationId) => _actThenRefresh('approve_application', {'id': applicationId});

  Future<RelayResponse> reject(String applicationId, {String reason = ''}) => _actThenRefresh('reject_application', {'id': applicationId, 'reason': reason});

  Future<RelayResponse> applyNow(String applicationId) => _actThenRefresh('apply_application', {'id': applicationId});

  Future<RelayResponse> answerQuestions(String applicationId, List<ApplicationAnswer> answers) => _actThenRefresh('answer_application_questions', {'id': applicationId, 'answers': answers.map((a) => a.toJson()).toList()});

  Future<RelayResponse> markManualComplete(String applicationId, {String note = ''}) => _actThenRefresh('mark_manual_application_complete', {'id': applicationId, 'note': note});

  Future<RelayResponse> setStatus(String applicationId, String status, {String note = ''}) => _actThenRefresh('set_application_status', {'id': applicationId, 'status': status, 'note': note});

  Future<RelayResponse> _actThenRefresh(String type, Map<String, dynamic> payload) async {
    final resp = await connection.request(type, payload);
    if (resp.ok) {
      unawaited(refresh());
    }
    return resp;
  }

  @override
  void dispose() {
    _changedSub?.cancel();
    _resyncSub?.cancel();
    super.dispose();
  }
}
