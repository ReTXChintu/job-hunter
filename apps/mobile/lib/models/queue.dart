import 'json.dart';

/// One task waiting for the desktop's agent (Rust `QueuedTask`, see
/// `docs/mobile-protocol.md` `list_queue`). Runs by itself, in order, once
/// the agent is free.
class QueuedTaskItem {
  final String id;
  final String title;

  /// `JOB_HUNT`, `APPLICATION`, `ANALYZE_JOB`, `GENERATE_RESUME`,
  /// `PROFILE_SYNC`, `JOB_LINK`, `SHARED_JOB` or `INBOX_CHECK`.
  final String kind;
  final DateTime? requestedAt;

  const QueuedTaskItem({required this.id, required this.title, required this.kind, this.requestedAt});

  factory QueuedTaskItem.fromJson(Map<String, dynamic> json) {
    final task = asMap(json['task']) ?? const {};
    return QueuedTaskItem(
      id: str(json['id']),
      title: str(json['title'], 'Queued task'),
      kind: str(task['kind']),
      requestedAt: dateOf(json['requestedAt']),
    );
  }
}

List<QueuedTaskItem> parseQueue(dynamic v) => mapList(v).map(QueuedTaskItem.fromJson).where((i) => i.id.isNotEmpty).toList();
