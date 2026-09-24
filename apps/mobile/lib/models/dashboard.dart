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
        jobsDiscovered: (json['jobsDiscovered'] as num?)?.toInt() ?? 0,
        relevant: (json['relevant'] as num?)?.toInt() ?? 0,
        awaitingApproval: (json['awaitingApproval'] as num?)?.toInt() ?? 0,
        applied: (json['applied'] as num?)?.toInt() ?? 0,
        manualAction: (json['manualAction'] as num?)?.toInt() ?? 0,
        waitingForUser: (json['waitingForUser'] as num?)?.toInt() ?? 0,
        interviews: (json['interviews'] as num?)?.toInt() ?? 0,
        rejected: (json['rejected'] as num?)?.toInt() ?? 0,
        offers: (json['offers'] as num?)?.toInt() ?? 0,
      );

  static const zero = DashboardCounts(jobsDiscovered: 0, relevant: 0, awaitingApproval: 0, applied: 0, manualAction: 0, waitingForUser: 0, interviews: 0, rejected: 0, offers: 0);
}

/// Trimmed `AgentStatus` the desktop pushes as `agent_status`: state and
/// whether it's paused, nothing about what it's doing in detail (see
/// `docs/mobile-protocol.md` -- the phone is a review surface, not a second
/// Agent Activity console).
class AgentStatusLite {
  final String state;
  final bool paused;
  final String? runKind;

  const AgentStatusLite({required this.state, required this.paused, this.runKind});

  factory AgentStatusLite.fromJson(Map<String, dynamic> json) => AgentStatusLite(
        state: json['state'] as String? ?? 'IDLE',
        paused: json['paused'] == true,
        runKind: json['runKind'] as String?,
      );

  static const idle = AgentStatusLite(state: 'IDLE', paused: false);
}

class Dashboard {
  final DashboardCounts today;
  final DashboardCounts total;
  final AgentStatusLite agent;

  const Dashboard({required this.today, required this.total, required this.agent});

  factory Dashboard.fromJson(Map<String, dynamic> json) => Dashboard(
        today: json['today'] is Map ? DashboardCounts.fromJson((json['today'] as Map).cast<String, dynamic>()) : DashboardCounts.zero,
        total: json['total'] is Map ? DashboardCounts.fromJson((json['total'] as Map).cast<String, dynamic>()) : DashboardCounts.zero,
        agent: json['agent'] is Map ? AgentStatusLite.fromJson((json['agent'] as Map).cast<String, dynamic>()) : AgentStatusLite.idle,
      );
}
