import 'package:flutter/material.dart';
import 'package:flutter_test/flutter_test.dart';
import 'package:job_hunter_mobile/screens/about_screen.dart';
import 'package:job_hunter_mobile/services/update_service.dart';
import 'package:job_hunter_mobile/state/auth_controller.dart';
import 'package:job_hunter_mobile/state/update_controller.dart';
import 'package:package_info_plus/package_info_plus.dart';
import 'package:provider/provider.dart';
import 'package:shared_preferences/shared_preferences.dart';

/// Answers like a server hosting build 9 of version 0.2.0.
class FakeUpdateService extends UpdateService {
  @override
  Future<UpdateInfo> check({required String serverUrl, required String currentVersion, required int currentBuild}) async {
    const latest = ServerBuild(fileName: 'job-hunter-0.2.0-9.apk', version: '0.2.0', build: 9, sizeBytes: 52428800, updatedAt: null, url: '/downloads/android');
    return UpdateInfo(currentVersion: currentVersion, currentBuild: currentBuild, latest: latest, available: isNewer(latest, currentVersion, currentBuild), downloadUrl: '$serverUrl/downloads/android');
  }
}

Future<UpdateController> pumpWith(WidgetTester tester, Widget child) async {
  SharedPreferences.setMockInitialValues({});
  PackageInfo.setMockInitialValues(appName: 'Job Hunter', packageName: 'io.jobhunter.job_hunter_mobile', version: '0.1.0', buildNumber: '3', buildSignature: '');
  final auth = AuthController()..relayUrl = 'http://10.0.0.5:8788';
  final updates = UpdateController(auth, service: FakeUpdateService());
  await tester.pumpWidget(ChangeNotifierProvider.value(value: updates, child: MaterialApp(home: child)));
  await tester.pumpAndSettle();
  return updates;
}

void main() {
  testWidgets('the banner announces a newer build and "Later" hides it', (tester) async {
    await pumpWith(tester, const Scaffold(body: Column(children: [UpdateBanner()])));
    expect(find.text('Job Hunter 0.2.0 is available'), findsOneWidget);
    expect(find.text('Download'), findsOneWidget);

    await tester.tap(find.byTooltip('Later'));
    await tester.pumpAndSettle();
    expect(find.text('Job Hunter 0.2.0 is available'), findsNothing);
    final prefs = await SharedPreferences.getInstance();
    expect(prefs.getString('job_hunter.update_dismissed'), startsWith('0.2.0|'));
  });

  testWidgets('About shows the installed version, the server, and the available update', (tester) async {
    await pumpWith(tester, const AboutScreen());
    expect(find.text('Version 0.1.0 (build 3)'), findsOneWidget);
    expect(find.text('http://10.0.0.5:8788'), findsOneWidget);
    expect(find.text('Version 0.2.0 (build 9) is available.'), findsOneWidget);
    expect(find.text('Download (50.0 MB)'), findsOneWidget);
    expect(find.text('Check for updates'), findsOneWidget);
  });
}
