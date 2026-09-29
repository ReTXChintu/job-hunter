import 'package:flutter/foundation.dart';

import '../logic/labels.dart';
import '../models/dashboard.dart';
import '../models/envelope.dart';
import 'connection_controller.dart';

/// The Home tab's quick actions: start or stop a job hunt, look for hiring
/// posts, check Gmail for replies. Each is one request the desktop runs
/// through its own orchestrator, exactly like its buttons.
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

  /// Why actions are unavailable right now, or null if they're available.
  String? get unavailableReason {
    if (!connection.connected) return notConnectedMessage;
    if (!connection.desktopOnline) return desktopOfflineMessage;
    return null;
  }

  Future<RelayResponse> startJobHunt() => _run('start', 'start_job_hunt', const {});

  Future<RelayResponse> findHiringPosts() => _run('posts', 'start_job_hunt', const {
        'sources': [hiringPostsSource],
      });

  Future<RelayResponse> stop() => _run('stop', 'stop_job_hunt', const {});

  Future<RelayResponse> checkInbox() => _run('inbox', 'check_inbox', const {});

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
