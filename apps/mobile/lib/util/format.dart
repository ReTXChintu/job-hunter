import 'package:flutter/material.dart';

/// "just now", "5 min ago", "3 h ago", "2 d ago".
String timeAgo(DateTime? at, {DateTime? now}) {
  if (at == null) return '';
  final diff = (now ?? DateTime.now()).difference(at);
  if (diff.inMinutes < 1) return 'just now';
  if (diff.inMinutes < 60) return '${diff.inMinutes} min ago';
  if (diff.inHours < 24) return '${diff.inHours} h ago';
  if (diff.inDays < 30) return '${diff.inDays} d ago';
  return '${(diff.inDays / 30).floor()} mo ago';
}

/// Local date and time in the device's format.
String formatDateTime(BuildContext context, DateTime? at) {
  if (at == null) return '';
  final local = at.toLocal();
  return '${MaterialLocalizations.of(context).formatShortDate(local)} ${TimeOfDay.fromDateTime(local).format(context)}';
}

String formatDate(BuildContext context, DateTime? at) => at == null ? '' : MaterialLocalizations.of(context).formatShortDate(at.toLocal());
