import 'dart:async';

import 'package:flutter/foundation.dart';

import '../logic/labels.dart';
import '../models/application.dart';
import '../models/envelope.dart';
import '../models/json.dart';
import '../models/profiles.dart';
import 'connection_controller.dart';
import 'data_source.dart';

/// Job-site profiles (More > Job sites): their state comes from the desktop
/// only (it isn't synced to the server), and updating one or answering its
/// questions runs on the desktop like its own Job sites page.
class ProfilesController extends ChangeNotifier {
  final ConnectionController connection;
  StreamSubscription? _changedSub;
  bool _active = false;

  ProfilesController(this.connection) {
    _changedSub = connection.changed.listen((u) {
      if (_active && u.collections.any((c) => c == 'platform_profiles' || c == 'agent_runs')) refresh();
    });
  }

  List<PlatformProfileView> items = [];
  bool loading = false;
  String? error;

  Future<void> refresh() async {
    _active = true;
    loading = true;
    notifyListeners();
    final resp = await connection.request('list_platform_profiles');
    if (resp.ok && resp.data is List) {
      items = mapList(resp.data).map(PlatformProfileView.fromJson).where((v) => v.profile.platform.isNotEmpty).toList();
      error = null;
    } else {
      error = friendlyError(resp);
    }
    loading = false;
    notifyListeners();
  }

  /// Update the profile; with [resume], continue the update that stopped
  /// part-way instead of starting over.
  Future<RelayResponse> sync(String platform, {bool resume = false}) => _act('sync_platform_profile', {'platform': platform, 'resume': resume});

  Future<RelayResponse> answer(String platform, List<ApplicationAnswer> answers) => _act('answer_platform_questions', {'platform': platform, 'answers': answers.map((a) => a.toJson()).toList()});

  Future<RelayResponse> _act(String type, Map<String, dynamic> payload) async {
    final resp = await connection.request(type, payload);
    if (resp.ok) unawaited(refresh());
    return resp;
  }

  @override
  void dispose() {
    _changedSub?.cancel();
    super.dispose();
  }
}

/// Saved answers to application questions (More > Additional details):
/// listed from the desktop, or read-only from the server while it's offline.
class AnswersController extends ChangeNotifier {
  final ConnectionController connection;
  StreamSubscription? _changedSub;
  bool _active = false;

  AnswersController(this.connection) {
    _changedSub = connection.changed.listen((u) {
      if (_active && u.collections.contains('application_answers')) refresh();
    });
  }

  List<AnswerRecord> items = [];
  bool loading = false;
  String? error;
  DataSource source = DataSource.none;

  Future<void> refresh() async {
    _active = true;
    loading = true;
    notifyListeners();
    final resp = await connection.request('list_answers');
    if (resp.ok && resp.data is List) {
      items = _sorted(mapList(resp.data).map(AnswerRecord.fromJson));
      source = DataSource.desktop;
      error = null;
    } else {
      final data = await connection.serverGet('/v1/data/application_answers');
      if (data is Map) {
        items = _sorted(mapList(data['documents']).map(AnswerRecord.fromJson));
        source = DataSource.server;
        error = null;
      } else {
        error = friendlyError(resp);
      }
    }
    loading = false;
    notifyListeners();
  }

  List<AnswerRecord> _sorted(Iterable<AnswerRecord> list) => list.where((a) => a.question.isNotEmpty).toList()..sort((a, b) => a.question.toLowerCase().compareTo(b.question.toLowerCase()));

  /// Creates (no [id]) or edits a saved answer on the desktop.
  Future<RelayResponse> save({String? id, required String question, required String answer}) async {
    final resp = await connection.request('save_answer', {if (id != null && id.isNotEmpty) 'id': id, 'question': question, 'answer': answer});
    if (resp.ok) unawaited(refresh());
    return resp;
  }

  @override
  void dispose() {
    _changedSub?.cancel();
    super.dispose();
  }
}
