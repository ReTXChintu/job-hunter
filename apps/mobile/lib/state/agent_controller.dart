import 'package:flutter/foundation.dart';

import '../logic/labels.dart';
import '../models/dashboard.dart';
import '../models/envelope.dart';
import 'connection_controller.dart';

/// The Home tab's actions: everything the desktop's own buttons start. Each
/// is one request the desktop runs through its own orchestrator; while the
/// agent is busy the desktop queues it and the reply says so.
class AgentController extends ChangeNotifier {
  final ConnectionController connection;
  AgentController(this.connection) {
    connection.addListener(notifyListeners);
  }

  /// The source name the desktop uses for LinkedIn hiring posts.
  static const hiringPostsSource = 'LinkedIn Posts';

  /// The action whose request is in flight, if any.
  String? running;

  AgentStatusLite? get status => connection.agentStatus;
  bool get busy => status != null && agentBusy(status!.state);

  /// Tasks waiting on the desktop.
  int get queued => status?.queued ?? 0;

  /// Why actions are unavailable right now, or null if they're available.
  String? get unavailableReason {
    if (!connection.connected) return notConnectedMessage;
    if (!connection.desktopOnline) return desktopOfflineMessage;
    return null;
  }

  Future<RelayResponse> startJobHunt() => _run('start', 'start_job_hunt', const {});

  /// Find and score jobs only; no applications are prepared.
  Future<RelayResponse> searchOnly() => _run('search', 'start_job_hunt', const {'discoverOnly': true});

  /// Applications for the best jobs already found, without a new search.
  Future<RelayResponse> prepareApplications() => _run('prepare', 'start_job_hunt', const {'prepareOnly': true});

  Future<RelayResponse> findHiringPosts() => _run('posts', 'start_job_hunt', const {
        'sources': [hiringPostsSource],
      });

  Future<RelayResponse> scanTelegram() => _run('telegram', 'start_job_hunt', const {
        'sources': ['Telegram'],
      });

  Future<RelayResponse> stop() => _run('stop', 'stop_job_hunt', const {});

  Future<RelayResponse> checkInbox() => _run('inbox', 'check_inbox', const {});

  /// Open a pasted job link on the desktop, prepare the application and,
  /// once the resume is ready, approve and apply.
  Future<RelayResponse> applyFromLink(String url) => _run('link', 'apply_from_link', {'url': url.trim(), 'applyNow': true});

  /// A job the user found as text: the desktop prepares the application
  /// and saves the email as a Gmail draft.
  Future<RelayResponse> addSharedJob(String text) => _run('shared', 'add_shared_job', {'text': text.trim(), 'draftEmail': true});

  Future<void> refreshStatus() => connection.refreshAgentStatus();

  Future<RelayResponse> _run(String name, String type, Map<String, dynamic> payload) async {
    running = name;
    notifyListeners();
    final resp = await connection.request(type, payload);
    running = null;
    notifyListeners();
    if (resp.ok) await connection.refreshAgentStatus();
    return resp;
  }

  @override
  void dispose() {
    connection.removeListener(notifyListeners);
    super.dispose();
  }
}
