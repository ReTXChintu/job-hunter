import 'dart:convert';

/// The one WebSocket frame shape spoken with the relay. Mirrors
/// `docs/mobile-protocol.md` and the Rust side in
/// `crates/job-hunter-core/src/remote/protocol.rs` -- keep all three in
/// sync by hand when the protocol changes.
class Envelope {
  final int v;
  final dynamic id;
  final String type;
  final Map<String, dynamic> payload;

  const Envelope({required this.v, required this.id, required this.type, required this.payload});

  factory Envelope.request(String id, String type, [Map<String, dynamic> payload = const {}]) => Envelope(v: 1, id: id, type: type, payload: payload);

  factory Envelope.fromJson(Map<String, dynamic> json) => Envelope(
        v: json['v'] as int? ?? 1,
        id: json['id'],
        type: json['type'] as String? ?? '',
        payload: (json['payload'] as Map?)?.cast<String, dynamic>() ?? const {},
      );

  Map<String, dynamic> toJson() => {'v': v, 'id': id, 'type': type, 'payload': payload};

  String encode() => jsonEncode(toJson());

  static Envelope decode(String text) => Envelope.fromJson(jsonDecode(text) as Map<String, dynamic>);
}

/// `payload` shape of a `response` envelope: `{ok, data}` or `{ok, error}`.
class RelayResponse {
  final bool ok;
  final dynamic data;
  final String? errorCode;
  final String? errorMessage;

  const RelayResponse({required this.ok, this.data, this.errorCode, this.errorMessage});

  factory RelayResponse.fromPayload(Map<String, dynamic> payload) {
    final ok = payload['ok'] == true;
    if (ok) return RelayResponse(ok: true, data: payload['data']);
    final error = (payload['error'] as Map?)?.cast<String, dynamic>();
    return RelayResponse(ok: false, errorCode: error?['code'] as String?, errorMessage: error?['message'] as String? ?? 'Something went wrong.');
  }
}

/// A `presence` push: `{"device":"desktop","online":bool,"lastSeenAt":iso}`.
class PresenceUpdate {
  final bool online;
  final DateTime? lastSeenAt;

  const PresenceUpdate({required this.online, this.lastSeenAt});

  factory PresenceUpdate.fromPayload(Map<String, dynamic> payload) => PresenceUpdate(
        online: payload['online'] == true,
        lastSeenAt: DateTime.tryParse(payload['lastSeenAt'] as String? ?? ''),
      );
}

/// A `changed` push: `{"collections": ["applications", "jobs"]}`.
class ChangedUpdate {
  final List<String> collections;
  const ChangedUpdate(this.collections);
  factory ChangedUpdate.fromPayload(Map<String, dynamic> payload) => ChangedUpdate(((payload['collections'] as List?) ?? const []).cast<String>());
}
