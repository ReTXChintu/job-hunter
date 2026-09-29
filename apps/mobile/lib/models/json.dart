/// Defensive JSON readers shared by the hand-written models. The desktop and
/// the server can be a version ahead of (or behind) this app, so every field
/// may be missing, null, or the wrong type: fall back to a default instead of
/// throwing, and never crash a whole list because one row is odd.
library;

Map<String, dynamic>? asMap(dynamic v) => v is Map ? v.cast<String, dynamic>() : null;

Map<String, dynamic> mapOrEmpty(dynamic v) => asMap(v) ?? const <String, dynamic>{};

List<Map<String, dynamic>> mapList(dynamic v) => v is List ? v.whereType<Map>().map((m) => m.cast<String, dynamic>()).toList() : const [];

List<String> strList(dynamic v) => v is List ? v.where((e) => e != null).map((e) => e.toString()).toList() : const [];

String str(dynamic v, [String fallback = '']) => v == null ? fallback : (v is String ? v : v.toString());

/// A string that means "absent" when null or empty.
String? optStr(dynamic v) {
  if (v == null) return null;
  final s = v is String ? v : v.toString();
  return s.isEmpty ? null : s;
}

int intOf(dynamic v, [int fallback = 0]) => v is num ? v.toInt() : (v is String ? int.tryParse(v) ?? fallback : fallback);

double? doubleOrNull(dynamic v) => v is num ? v.toDouble() : (v is String ? double.tryParse(v) : null);

bool boolOf(dynamic v) => v == true || v == 'true';

DateTime? dateOf(dynamic v) => v is String && v.isNotEmpty ? DateTime.tryParse(v) : null;

/// Epoch zero, for sorting rows with no timestamp last.
final DateTime epoch = DateTime.fromMillisecondsSinceEpoch(0, isUtc: true);
