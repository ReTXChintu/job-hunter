/// The Job Hunter server this build talks to, fixed at build time:
///
///   flutter build apk --release --dart-define=BACKEND_URL=http://1.2.3.4:8788
///
/// CI sets it from the `BACKEND_URL` GitHub secret (see
/// `.github/workflows/build.yml`). When it's set, the app has no server
/// URL field at all. A local `flutter run` without it falls back to asking
/// for one, which is handy for development only.
const String kBuiltInServerUrl = String.fromEnvironment('BACKEND_URL');

bool get hasBuiltInServer => kBuiltInServerUrl.trim().isNotEmpty;
