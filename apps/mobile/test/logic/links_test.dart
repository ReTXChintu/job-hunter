import 'package:flutter_test/flutter_test.dart';
import 'package:job_hunter_mobile/logic/links.dart';
import 'package:job_hunter_mobile/models/app_notification.dart';

void main() {
  test('notification links open the right screen', () {
    expect(resolveLink('application', 'app-1'), const AppLink(LinkKind.application, 'app-1'));
    expect(resolveLink('application', ''), const AppLink(LinkKind.applications), reason: 'no id: the list');
    expect(resolveLink('applications', null), const AppLink(LinkKind.applications));
    expect(resolveLink('jobs', null), const AppLink(LinkKind.jobs));
    expect(resolveLink('job-sites', 'LinkedIn'), const AppLink(LinkKind.jobSites, null));
    expect(resolveLink('agent', null), const AppLink(LinkKind.home));
    expect(resolveLink(null, null), const AppLink(LinkKind.home));
    expect(resolveLink('something-new', 'x'), const AppLink(LinkKind.home));
  });

  test('each link lands on its tab', () {
    expect(tabFor(const AppLink(LinkKind.application, 'a')), tabApplications);
    expect(tabFor(const AppLink(LinkKind.applications)), tabApplications);
    expect(tabFor(const AppLink(LinkKind.jobs)), tabJobs);
    expect(tabFor(const AppLink(LinkKind.jobSites)), tabMore);
    expect(tabFor(const AppLink(LinkKind.home)), tabHome);
  });

  test('local notification payloads round-trip, and old bare ids still open the application', () {
    expect(decodeLinkPayload(encodeLinkPayload('application', 'app-9')), const AppLink(LinkKind.application, 'app-9'));
    expect(decodeLinkPayload(encodeLinkPayload('job-sites', null)), const AppLink(LinkKind.jobSites));
    expect(decodeLinkPayload('app-legacy'), const AppLink(LinkKind.application, 'app-legacy'));
    expect(decodeLinkPayload(null), isNull);
    expect(decodeLinkPayload(''), isNull);
  });

  test('Firebase data (string values, empty = absent) maps to links and notifications', () {
    final data = {'id': 'n1', 'kind': 'APPLICATION_NEEDS_INPUT', 'level': 'WARN', 'linkPage': 'application', 'linkId': 'app-3', 'createdAt': '2026-09-29T10:00:00Z'};
    expect(linkFromPushData(data), const AppLink(LinkKind.application, 'app-3'));
    expect(linkFromPushData({'linkPage': 'jobs', 'linkId': ''}), const AppLink(LinkKind.jobs));

    final n = AppNotification.fromPushData({...data, 'linkId': ''}, title: 'Needs you', body: 'Answer 2 questions');
    expect(n.title, 'Needs you');
    expect(n.linkId, isNull);
    expect(n.applicationId, isNull);
    expect(n.link, const AppLink(LinkKind.applications));
    expect(AppNotification.fromPushData(data).link, const AppLink(LinkKind.application, 'app-3'));
  });
}
