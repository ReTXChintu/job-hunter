import '../logic/links.dart';
import 'json.dart';

/// A notification the desktop raised (a job hunt finished, an application
/// or a job-site update needs you, something failed). Mirrors the Rust
/// `domain::Notification` / `packages/types` `Notification`, trimmed to what
/// the phone shows.
class AppNotification {
  final String id;
  final String level; // INFO | SUCCESS | WARN | ERROR
  final String kind;
  final String title;
  final String body;
  final String linkPage; // "application" | "applications" | "job-sites" | "jobs" | "agent"
  final String? linkId;
  final String createdAt; // ISO 8601, compared as a string

  const AppNotification({
    required this.id,
    required this.level,
    required this.kind,
    required this.title,
    required this.body,
    required this.linkPage,
    required this.linkId,
    required this.createdAt,
  });

  factory AppNotification.fromJson(Map<String, dynamic> j) => AppNotification(
        id: j['id'] as String? ?? '',
        level: j['level'] as String? ?? 'INFO',
        kind: j['kind'] as String? ?? '',
        title: j['title'] as String? ?? '',
        body: j['body'] as String? ?? '',
        linkPage: j['linkPage'] as String? ?? '',
        linkId: j['linkId'] as String?,
        createdAt: j['createdAt'] as String? ?? '',
      );

  /// A Firebase message: `data` carries the fields (string values, empty
  /// meaning absent), `notification` the title and body.
  factory AppNotification.fromPushData(Map<String, dynamic> data, {String? title, String? body}) => AppNotification(
        id: str(data['id']),
        level: str(data['level'], 'INFO'),
        kind: str(data['kind']),
        title: title ?? str(data['title']),
        body: body ?? str(data['body']),
        linkPage: str(data['linkPage']),
        linkId: optStr(data['linkId']),
        createdAt: str(data['createdAt']),
      );

  /// The application this opens, if it links to one.
  String? get applicationId => linkPage == 'application' && linkId != null && linkId!.isNotEmpty ? linkId : null;

  /// Where tapping it goes.
  AppLink get link => resolveLink(linkPage, linkId);
}

/// Newest first.
List<AppNotification> sortNewest(Iterable<AppNotification> list) => list.toList()..sort((a, b) => b.createdAt.compareTo(a.createdAt));

/// The notifications newer than [alertedAt], oldest first. With no
/// [alertedAt] (first check on this phone) nothing is fresh, so installing
/// the app doesn't replay history as alerts.
List<AppNotification> freshSince(List<AppNotification> list, String? alertedAt) {
  if (alertedAt == null) return const [];
  return list.where((n) => n.createdAt.compareTo(alertedAt) > 0).toList()..sort((a, b) => a.createdAt.compareTo(b.createdAt));
}

/// The newest `createdAt` among [list] and [current].
String? newestTimestamp(List<AppNotification> list, String? current) {
  var latest = current;
  for (final n in list) {
    if (latest == null || n.createdAt.compareTo(latest) > 0) latest = n.createdAt;
  }
  return latest;
}
