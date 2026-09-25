import 'package:flutter_test/flutter_test.dart';
import 'package:job_hunter_mobile/services/update_service.dart';

ServerBuild build(String? version, int? number) =>
    ServerBuild(fileName: 'job-hunter.apk', version: version, build: number, sizeBytes: 1, updatedAt: null, url: '/downloads/android');

void main() {
  test('compareVersions orders like semver', () {
    expect(compareVersions('0.2.0', '0.1.9'), greaterThan(0));
    expect(compareVersions('0.10.0', '0.9.0'), greaterThan(0));
    expect(compareVersions('1.0.0', '1.0.0'), 0);
    expect(compareVersions('v1.2.3', '1.2.3'), 0);
    expect(compareVersions('1.0.0-rc.1', '1.0.0'), lessThan(0));
    expect(compareVersions('garbage', '1.0.0'), 0);
  });

  test('a newer version, or the same version with a higher build number, is an update', () {
    expect(isNewer(build('0.2.0', 5), '0.1.0', 40), isTrue);
    expect(isNewer(build('0.2.0', 41), '0.2.0', 40), isTrue);
    expect(isNewer(build('0.2.0', 40), '0.2.0', 40), isFalse);
    expect(isNewer(build('0.2.0', null), '0.2.0', 40), isFalse);
    expect(isNewer(build('0.1.0', 99), '0.2.0', 1), isFalse);
    expect(isNewer(build(null, 99), '0.1.0', 1), isFalse);
  });

  test('ServerBuild parses the backend response', () {
    final b = ServerBuild.fromJson({'fileName': 'job-hunter-0.2.0-7.apk', 'version': '0.2.0', 'build': 7, 'sizeBytes': 123, 'updatedAt': '2026-09-25T10:00:00.000Z', 'url': '/downloads/android'});
    expect(b.version, '0.2.0');
    expect(b.build, 7);
    expect(b.updatedAt, isNotNull);
  });
}
