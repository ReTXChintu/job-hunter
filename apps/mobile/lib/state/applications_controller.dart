import 'dart:async';
import 'dart:convert';

import 'package:flutter/foundation.dart';
import 'package:shared_preferences/shared_preferences.dart';

import '../models/application.dart';
import '../models/dashboard.dart';
import '../models/envelope.dart';
import 'connection_controller.dart';

/// The review list/detail data and the actions a phone may take on it.
/// Every action is one `connection.request(...)` call that reaches the
/// exact same desktop-side `orchestrator` function the Tauri UI calls (see
/// `crates/job-hunter-core/src/remote/dispatch.rs`) -- this class never
/// decides an application's fate on its own, it only asks the desktop to.
class ApplicationsController extends ChangeNotifier {
  static const _cacheKey = 'job_hunter.applications_cache';

  final ConnectionController connection;
  StreamSubscription? _changedSub;

  ApplicationsController(this.connection) {
    _changedSub = connection.changed.listen((update) {
      if (update.collections.any((c) => c == 'applications' || c == 'jobs' || c == 'job_analyses' || c == 'agent_runs')) {
        refresh();
      }
    });
    _loadCache();
  }

  List<ApplicationListItem> items = [];
  Dashboard? dashboard;
  bool loading = false;
  bool isFromCache = false;
  String? error;
  DateTime? lastLoadedAt;

  Future<void> _loadCache() async {
    final prefs = await SharedPreferences.getInstance();
    final raw = prefs.getString(_cacheKey);
    if (raw == null) return;
    try {
      final list = (jsonDecode(raw) as List).map((e) => ApplicationListItem.fromJson((e as Map).cast<String, dynamic>())).toList();
      items = list;
      isFromCache = true;
      notifyListeners();
    } catch (_) {
      // Corrupt cache: ignore, a live refresh will replace it.
    }
  }

  Future<void> _saveCache(List<ApplicationListItem> list) async {
    final prefs = await SharedPreferences.getInstance();
    final raw = jsonEncode(list.map((i) => {'application': _applicationJson(i.application), 'job': _jobJson(i.job)}).toList());
    await prefs.setString(_cacheKey, raw);
  }

  Future<void> refresh() async {
    loading = true;
    notifyListeners();
    final resp = await connection.request('list_applications');
    if (resp.ok && resp.data is List) {
      items = (resp.data as List).map((e) => ApplicationListItem.fromJson((e as Map).cast<String, dynamic>())).toList();
      isFromCache = false;
      lastLoadedAt = DateTime.now();
      error = null;
      unawaited(_saveCache(items));
    } else if (!resp.ok) {
      error = resp.errorMessage;
    }
    final dashResp = await connection.request('list_dashboard');
    if (dashResp.ok && dashResp.data is Map) {
      dashboard = Dashboard.fromJson((dashResp.data as Map).cast<String, dynamic>());
    }
    loading = false;
    notifyListeners();
  }

  Future<ApplicationDetail?> loadDetail(String applicationId) async {
    final resp = await connection.request('get_application', {'id': applicationId});
    if (resp.ok && resp.data is Map) {
      return ApplicationDetail.fromJson((resp.data as Map).cast<String, dynamic>());
    }
    error = resp.errorMessage;
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

  Map<String, dynamic> _applicationJson(Application a) => {
        'id': a.id,
        'jobId': a.jobId,
        'status': a.status,
        'applicationUrl': a.applicationUrl,
        'notes': a.notes,
        'answers': a.answers.map((x) => {'question': x.question, 'answer': x.answer, 'source': x.source}).toList(),
        'pendingQuestions': const [],
        'statusHistory': const [],
        'potentialIssues': a.potentialIssues,
        'approvedAt': a.approvedAt?.toIso8601String(),
        'appliedAt': a.appliedAt?.toIso8601String(),
        'failureReason': a.failureReason,
        'evidence': a.evidence,
        'manualCompleted': a.manualCompleted,
        'updatedAt': a.updatedAt.toIso8601String(),
      };

  Map<String, dynamic> _jobJson(dynamic job) => {
        'id': job.id,
        'title': job.title,
        'company': job.company,
        'location': job.location,
        'employmentType': job.employmentType,
        'remote': job.remote,
        'salary': job.salary,
        'seniority': job.seniority,
        'postedAt': job.postedAt,
        'source': job.source,
        'url': job.url,
        'description': job.description,
        'requirements': job.requirements,
        'responsibilities': job.responsibilities,
        'skills': job.skills,
        'status': job.status,
      };

  @override
  void dispose() {
    _changedSub?.cancel();
    super.dispose();
  }
}
