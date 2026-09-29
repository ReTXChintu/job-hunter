import 'json.dart';

class DashboardCounts {
  final int jobsDiscovered;
  final int relevant;
  final int awaitingApproval;
  final int applied;
  final int manualAction;
  final int waitingForUser;
  final int interviews;
  final int rejected;
  final int offers;

  const DashboardCounts({
    required this.jobsDiscovered,
    required this.relevant,
    required this.awaitingApproval,
    required this.applied,
    required this.manualAction,
    required this.waitingForUser,
    required this.interviews,
    required this.rejected,
    required this.offers,
  });

  factory DashboardCounts.fromJson(Map<String, dynamic> json) => DashboardCounts(
        jobsDiscovered: intOf(json['jobsDiscovered']),
        relevant: intOf(json['relevant']),
        awaitingApproval: intOf(json['awaitingApproval']),
        applied: intOf(json['applied']),
        manualAction: intOf(json['manualAction']),
        waitingForUser: intOf(json['waitingForUser']),
        interviews: intOf(json['interviews']),
        rejected: intOf(json['rejected']),
        offers: intOf(json['offers']),
      );

  static const zero = DashboardCounts(jobsDiscovered: 0, relevant: 0, awaitingApproval: 0, applied: 0, manualAction: 0, waitingForUser: 0, interviews: 0, rejected: 0, offers: 0);
}

/// Trimmed `AgentStatus` the desktop pushes as `agent_status` (and returns
/// from `get_agent_status`): state, whether it's paused, what kind of run,
/// and progress when known. Never the free-text activity (see
/// `docs/mobile-protocol.md`).
class AgentStatusLite {
  final String state;
  final bool paused;
  final String? runKind;
  final double? progress;

  const AgentStatusLite({required this.state, required this.paused, this.runKind, this.progress});

  factory AgentStatusLite.fromJson(Map<String, dynamic> json) => AgentStatusLite(
        state: str(json['state'], 'IDLE'),
        paused: boolOf(json['paused']),
        runKind: optStr(json['runKind']),
        progress: doubleOrNull(json['progress']),
      );

  static const idle = AgentStatusLite(state: 'IDLE', paused: false);
}

class Dashboard {
  final DashboardCounts today;
  final DashboardCounts total;
  final AgentStatusLite agent;

  const Dashboard({required this.today, required this.total, required this.agent});

  factory Dashboard.fromJson(Map<String, dynamic> json) => Dashboard(
        today: asMap(json['today']) == null ? DashboardCounts.zero : DashboardCounts.fromJson(asMap(json['today'])!),
        total: asMap(json['total']) == null ? DashboardCounts.zero : DashboardCounts.fromJson(asMap(json['total'])!),
        agent: asMap(json['agent']) == null ? AgentStatusLite.idle : AgentStatusLite.fromJson(asMap(json['agent'])!),
      );
}
