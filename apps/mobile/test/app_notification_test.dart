import 'package:flutter_test/flutter_test.dart';
import 'package:job_hunter_mobile/models/app_notification.dart';

AppNotification n(String id, String createdAt, {String linkPage = 'applications', String? linkId}) =>
    AppNotification(id: id, level: 'INFO', kind: 'X', title: id, body: '', linkPage: linkPage, linkId: linkId, createdAt: createdAt);

void main() {
  test('nothing is fresh on the first check, so history is not replayed', () {
    expect(freshSince([n('a', '2026-09-28T10:00:00Z')], null), isEmpty);
  });

  test('only notifications after the watermark are fresh, oldest first', () {
    final list = [n('c', '2026-09-28T12:00:00Z'), n('a', '2026-09-28T10:00:00Z'), n('b', '2026-09-28T11:00:00Z')];
    expect(freshSince(list, '2026-09-28T10:00:00Z').map((x) => x.id), ['b', 'c']);
    expect(newestTimestamp(list, '2026-09-28T10:00:00Z'), '2026-09-28T12:00:00Z');
    expect(newestTimestamp(const [], null), isNull);
  });

  test('only application notifications open on the phone', () {
    expect(n('a', 't', linkPage: 'application', linkId: 'app-1').applicationId, 'app-1');
    expect(n('a', 't', linkPage: 'job-sites', linkId: 'linkedin').applicationId, isNull);
  });

  test('parses the server shape', () {
    final parsed = AppNotification.fromJson({'id': 'n1', 'level': 'ERROR', 'title': 'LinkedIn profile update stopped', 'linkPage': 'job-sites', 'createdAt': '2026-09-28T10:00:00Z'});
    expect(parsed.level, 'ERROR');
    expect(parsed.body, '');
    expect(parsed.linkId, isNull);
  });
}
