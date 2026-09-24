import 'package:flutter_test/flutter_test.dart';
import 'package:job_hunter_mobile/models/envelope.dart';

void main() {
  group('Envelope', () {
    test('request() builds a v1 envelope and round-trips through encode/decode', () {
      final env = Envelope.request('abc123', 'approve_application', {'id': 'app-1'});
      expect(env.v, 1);
      expect(env.id, 'abc123');
      expect(env.type, 'approve_application');
      expect(env.payload, {'id': 'app-1'});

      final decoded = Envelope.decode(env.encode());
      expect(decoded.v, 1);
      expect(decoded.id, 'abc123');
      expect(decoded.type, 'approve_application');
      expect(decoded.payload, {'id': 'app-1'});
    });

    test('fromJson defaults a missing payload to an empty map, not null', () {
      final env = Envelope.fromJson({'v': 1, 'id': 'x', 'type': 'presence'});
      expect(env.payload, <String, dynamic>{});
    });

    test('fromJson defaults a missing v to 1', () {
      final env = Envelope.fromJson({'id': 'x', 'type': 'presence', 'payload': {}});
      expect(env.v, 1);
    });
  });

  group('RelayResponse', () {
    test('parses an ok=true payload as success with data', () {
      final resp = RelayResponse.fromPayload({'ok': true, 'data': {'status': 'APPROVED'}});
      expect(resp.ok, isTrue);
      expect(resp.data, {'status': 'APPROVED'});
      expect(resp.errorCode, isNull);
    });

    test('parses an ok=false payload as failure with code/message', () {
      final resp = RelayResponse.fromPayload({
        'ok': false,
        'error': {'code': 'DESKTOP_OFFLINE', 'message': 'Desktop is not connected.'},
      });
      expect(resp.ok, isFalse);
      expect(resp.errorCode, 'DESKTOP_OFFLINE');
      expect(resp.errorMessage, 'Desktop is not connected.');
    });

    test('falls back to a generic message when the error has none', () {
      final resp = RelayResponse.fromPayload({'ok': false, 'error': {'code': 'UNKNOWN'}});
      expect(resp.errorMessage, 'Something went wrong.');
    });
  });

  group('PresenceUpdate', () {
    test('parses online + lastSeenAt', () {
      final update = PresenceUpdate.fromPayload({'device': 'desktop', 'online': true, 'lastSeenAt': '2026-01-01T00:00:00.000Z'});
      expect(update.online, isTrue);
      expect(update.lastSeenAt, DateTime.parse('2026-01-01T00:00:00.000Z'));
    });

    test('missing lastSeenAt parses to null rather than throwing', () {
      final update = PresenceUpdate.fromPayload({'online': false});
      expect(update.online, isFalse);
      expect(update.lastSeenAt, isNull);
    });
  });

  group('ChangedUpdate', () {
    test('extracts the collections list', () {
      final update = ChangedUpdate.fromPayload({'collections': ['applications', 'jobs']});
      expect(update.collections, ['applications', 'jobs']);
    });

    test('missing collections defaults to empty, not null', () {
      final update = ChangedUpdate.fromPayload({});
      expect(update.collections, isEmpty);
    });
  });
}
