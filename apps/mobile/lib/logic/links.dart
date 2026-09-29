import 'dart:convert';

/// Where tapping a notification (in the Alerts tab, a system notification,
/// or a Firebase push) takes the user.
enum LinkKind { application, applications, jobs, jobSites, home }

class AppLink {
  final LinkKind kind;
  final String? id;
  const AppLink(this.kind, [this.id]);

  @override
  bool operator ==(Object other) => other is AppLink && other.kind == kind && other.id == id;

  @override
  int get hashCode => Object.hash(kind, id);

  @override
  String toString() => 'AppLink($kind${id == null ? '' : ', $id'})';
}

/// Maps a notification's `linkPage`/`linkId` (see `packages/types`
/// `Notification`) to a screen. Anything unknown opens Home.
AppLink resolveLink(String? linkPage, String? linkId) {
  final id = (linkId == null || linkId.isEmpty) ? null : linkId;
  switch (linkPage) {
    case 'application':
      return id == null ? const AppLink(LinkKind.applications) : AppLink(LinkKind.application, id);
    case 'applications':
      return const AppLink(LinkKind.applications);
    case 'jobs':
    case 'job':
      return const AppLink(LinkKind.jobs);
    case 'job-sites':
    case 'job_sites':
      return const AppLink(LinkKind.jobSites);
    default:
      return const AppLink(LinkKind.home);
  }
}

/// The payload stored on a local notification, read back when it's tapped.
String encodeLinkPayload(String linkPage, String? linkId) => jsonEncode({'linkPage': linkPage, 'linkId': linkId});

/// Reads [encodeLinkPayload]'s output. Older builds stored just the
/// application id, so a bare non-JSON string still opens that application.
AppLink? decodeLinkPayload(String? payload) {
  if (payload == null || payload.isEmpty) return null;
  try {
    final decoded = jsonDecode(payload);
    if (decoded is Map) return resolveLink(decoded['linkPage']?.toString(), decoded['linkId']?.toString());
  } catch (_) {
    // not JSON: the legacy application-id payload
  }
  return AppLink(LinkKind.application, payload);
}

/// A Firebase message's `data` map (string values; empty means absent).
AppLink linkFromPushData(Map<String, dynamic> data) => resolveLink(data['linkPage']?.toString(), data['linkId']?.toString());

/// Tab order in the signed-in shell.
const tabHome = 0, tabApplications = 1, tabJobs = 2, tabAlerts = 3, tabMore = 4;

/// The tab a link lands on (an application detail opens over Applications,
/// job sites over More).
int tabFor(AppLink link) => switch (link.kind) {
      LinkKind.application || LinkKind.applications => tabApplications,
      LinkKind.jobs => tabJobs,
      LinkKind.jobSites => tabMore,
      LinkKind.home => tabHome,
    };
