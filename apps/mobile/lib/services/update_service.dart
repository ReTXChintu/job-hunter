import 'package:dio/dio.dart';

/// The latest Android build the Job Hunter server hosts, from
/// `GET /v1/downloads` (see `apps/backend/src/routes/static.ts`). No auth:
/// the build list isn't secret, so this works even while signed out.
class ServerBuild {
  final String fileName;
  final String? version;
  final int? build;
  final int sizeBytes;
  final DateTime? updatedAt;
  final String url;

  const ServerBuild({required this.fileName, required this.version, required this.build, required this.sizeBytes, required this.updatedAt, required this.url});

  factory ServerBuild.fromJson(Map<String, dynamic> json) => ServerBuild(
        fileName: json['fileName'] as String? ?? '',
        version: json['version'] as String?,
        build: (json['build'] as num?)?.toInt(),
        sizeBytes: (json['sizeBytes'] as num?)?.toInt() ?? 0,
        updatedAt: DateTime.tryParse(json['updatedAt'] as String? ?? ''),
        url: json['url'] as String? ?? '/downloads/android',
      );
}

class UpdateInfo {
  final String currentVersion;
  final int currentBuild;
  final ServerBuild? latest;
  final bool available;
  final String? downloadUrl;

  const UpdateInfo({required this.currentVersion, required this.currentBuild, required this.latest, required this.available, required this.downloadUrl});
}

/// Semver ordering; a prerelease sorts before its release. Unparseable
/// versions compare equal, so garbage never looks like an update.
int compareVersions(String a, String b) {
  List<int>? core(String v) {
    final parts = v.trim().replaceFirst(RegExp('^v'), '').split('-').first.split('.');
    if (parts.length != 3) return null;
    final nums = parts.map(int.tryParse).toList();
    return nums.contains(null) ? null : nums.cast<int>();
  }

  String? pre(String v) {
    final i = v.indexOf('-');
    return i < 0 ? null : v.substring(i + 1);
  }

  final ca = core(a), cb = core(b);
  if (ca == null || cb == null) return 0;
  for (var i = 0; i < 3; i++) {
    if (ca[i] != cb[i]) return ca[i].compareTo(cb[i]);
  }
  final pa = pre(a), pb = pre(b);
  if (pa == null && pb == null) return 0;
  if (pa == null) return 1;
  if (pb == null) return -1;
  return pa.compareTo(pb);
}

/// Whether [latest] is newer than the installed version/build. A rebuild of
/// the same version with a higher build number (CI's run number) counts.
bool isNewer(ServerBuild latest, String currentVersion, int currentBuild) {
  final version = latest.version;
  if (version == null) return false;
  final cmp = compareVersions(version, currentVersion);
  if (cmp != 0) return cmp > 0;
  return latest.build != null && latest.build! > currentBuild;
}

class UpdateService {
  final Dio _dio;
  UpdateService({Dio? dio}) : _dio = dio ?? Dio(BaseOptions(connectTimeout: const Duration(seconds: 10), receiveTimeout: const Duration(seconds: 10)));

  Future<UpdateInfo> check({required String serverUrl, required String currentVersion, required int currentBuild}) async {
    final base = serverUrl.endsWith('/') ? serverUrl.substring(0, serverUrl.length - 1) : serverUrl;
    final resp = await _dio.get<Map<String, dynamic>>('$base/v1/downloads');
    final raw = resp.data?['android'];
    final latest = raw is Map<String, dynamic> ? ServerBuild.fromJson(raw) : null;
    final available = latest != null && isNewer(latest, currentVersion, currentBuild);
    return UpdateInfo(
      currentVersion: currentVersion,
      currentBuild: currentBuild,
      latest: latest,
      available: available,
      downloadUrl: latest == null ? null : '$base${latest.url}',
    );
  }
}
