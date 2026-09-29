import 'dart:async';

import 'package:flutter/foundation.dart';

import '../logic/labels.dart';
import '../models/application.dart';
import '../models/envelope.dart';
import '../models/job.dart';
import '../models/json.dart';
import 'connection_controller.dart';
import 'data_source.dart';

/// Every job the desktop found, with its analysis. Read from the desktop
/// (`list_jobs` / `get_job`) or, while it's offline, rebuilt from the
/// server's synced collections. Actions (prepare an application, reject a
/// job) always go to the desktop.
class JobsController extends ChangeNotifier {
  final ConnectionController connection;
  StreamSubscription? _changedSub;
  StreamSubscription? _resyncSub;
  bool _loadedOnce = false;

  JobsController(this.connection) {
    _changedSub = connection.changed.listen((update) {
      if (_loadedOnce && update.collections.any((c) => const {'jobs', 'job_analyses', 'applications'}.contains(c))) refresh();
    });
    _resyncSub = connection.resync.listen((_) {
      if (_loadedOnce) refresh();
    });
  }

  List<JobListItem> items = [];
  bool loading = false;
  DataSource source = DataSource.none;
  String? error;

  bool get isFromServer => source == DataSource.server;

  /// Loads once when the Jobs tab is first shown.
  Future<void> ensureLoaded() async {
    if (_loadedOnce) return;
    await refresh();
  }

  Future<void> refresh() async {
    _loadedOnce = true;
    loading = true;
    notifyListeners();
    final resp = await connection.request('list_jobs');
    if (resp.ok && resp.data is List) {
      items = mapList(resp.data).map(JobListItem.fromJson).where((i) => i.job.id.isNotEmpty).toList();
      source = DataSource.desktop;
      error = null;
    } else {
      final results = await Future.wait([
        connection.serverGet('/v1/data/jobs'),
        connection.serverGet('/v1/data/job_analyses'),
        connection.serverGet('/v1/applications'),
      ]);
      if (results[0] is Map) {
        items = buildJobItemsFromServer(results[0], results[1], results[2]);
        source = DataSource.server;
        error = null;
      } else {
        error = resp.ok ? 'Unexpected reply from the desktop.' : friendlyError(resp);
      }
    }
    loading = false;
    notifyListeners();
  }

  String? lastDetailError;
  bool lastDetailFromServer = false;

  Future<JobDetail?> loadDetail(String jobId) async {
    final resp = await connection.request('get_job', {'id': jobId});
    if (resp.ok && resp.data is Map) {
      lastDetailFromServer = false;
      lastDetailError = null;
      return JobDetail.fromJson((resp.data as Map).cast<String, dynamic>());
    }
    // Offline: what the list already has, plus the application from the server.
    final item = items.where((i) => i.job.id == jobId).firstOrNull;
    if (item != null) {
      Application? application;
      final appId = item.job.applicationId;
      if (appId != null) {
        final data = await connection.serverGet('/v1/applications/${Uri.encodeComponent(appId)}');
        if (data is Map && data['application'] is Map) application = Application.fromJson((data['application'] as Map).cast<String, dynamic>());
      }
      lastDetailFromServer = true;
      lastDetailError = null;
      return JobDetail(job: item.job, analysis: item.analysis, application: application);
    }
    lastDetailError = resp.ok ? 'Could not load this job.' : friendlyError(resp);
    return null;
  }

  /// Asks the desktop to tailor a resume (and cover letter) for this job,
  /// which creates its application for review.
  Future<RelayResponse> prepareApplication(String jobId) => _act('generate_resume', {'jobId': jobId});

  Future<RelayResponse> rejectJob(String jobId) => _act('reject_job', {'id': jobId});

  Future<RelayResponse> _act(String type, Map<String, dynamic> payload) async {
    final resp = await connection.request(type, payload);
    if (resp.ok) unawaited(refresh());
    return resp;
  }

  @override
  void dispose() {
    _changedSub?.cancel();
    _resyncSub?.cancel();
    super.dispose();
  }
}
