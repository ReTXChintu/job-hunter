import 'package:dio/dio.dart';

/// Thrown for any failed relay REST call, carrying the same `{code,
/// message}` shape the desktop's `UserFacingError` uses, so error text is
/// consistent wherever it's shown.
class RelayApiException implements Exception {
  final String code;
  final String message;
  const RelayApiException(this.code, this.message);
  @override
  String toString() => message;
}

class DeviceRegistration {
  final String deviceId;
  final String deviceToken;
  const DeviceRegistration({required this.deviceId, required this.deviceToken});
}

/// REST calls to a `job-hunter-relay` instance: accounts, device
/// registration, and pairing. See `docs/mobile-protocol.md`. This app never
/// calls anything else on the relay, and never talks to the desktop's
/// MongoDB, Claude, or Chrome directly.
class ApiClient {
  final Dio _dio;

  ApiClient(String baseUrl) : _dio = Dio(BaseOptions(baseUrl: baseUrl, connectTimeout: const Duration(seconds: 15), receiveTimeout: const Duration(seconds: 15)));

  static String normalizeRelayUrl(String input) {
    final trimmed = input.trim();
    if (trimmed.endsWith('/')) return trimmed.substring(0, trimmed.length - 1);
    return trimmed;
  }

  Future<T> _post<T>(String path, Map<String, dynamic> data, T Function(Map<String, dynamic>) parse, {String? bearer}) async {
    try {
      final resp = await _dio.post<Map<String, dynamic>>(
        path,
        data: data,
        options: bearer == null ? null : Options(headers: {'Authorization': 'Bearer $bearer'}),
      );
      return parse(resp.data ?? const {});
    } on DioException catch (e) {
      throw _translate(e);
    }
  }

  RelayApiException _translate(DioException e) {
    final body = e.response?.data;
    if (body is Map && body['error'] is Map) {
      final err = (body['error'] as Map).cast<String, dynamic>();
      return RelayApiException(err['code'] as String? ?? 'ERROR', err['message'] as String? ?? 'Something went wrong.');
    }
    if (e.type == DioExceptionType.connectionTimeout || e.type == DioExceptionType.connectionError) {
      return const RelayApiException('NETWORK', "Couldn't reach the relay server. Check the address and your connection.");
    }
    return RelayApiException('NETWORK', e.message ?? 'Something went wrong.');
  }

  /// Returns a short-lived access token, used only to immediately register
  /// this device -- never stored.
  Future<String> register(String email, String password) => _post('/v1/auth/register', {'email': email, 'password': password}, (j) => j['accessToken'] as String);

  Future<String> login(String email, String password) => _post('/v1/auth/login', {'email': email, 'password': password}, (j) => j['accessToken'] as String);

  Future<DeviceRegistration> registerDevice({required String accessToken, required String name, required String platform}) => _post(
        '/v1/devices/register',
        {'name': name, 'kind': 'mobile', 'platform': platform},
        (j) => DeviceRegistration(deviceId: j['deviceId'] as String, deviceToken: j['deviceToken'] as String),
        bearer: accessToken,
      );

  Future<DeviceRegistration> redeemPairingCode({required String code, required String name, required String platform}) => _post(
        '/v1/pairing/redeem',
        {'code': code, 'name': name, 'platform': platform},
        (j) => DeviceRegistration(deviceId: j['deviceId'] as String, deviceToken: j['deviceToken'] as String),
      );

  /// Revokes this phone's own device entry, using its own device token as
  /// authorization (the relay accepts a device token anywhere it accepts an
  /// access token -- see `docs/mobile-protocol.md`).
  Future<void> revokeThisDevice({required String deviceToken, required String deviceId}) async {
    try {
      await _dio.delete<void>('/v1/devices/$deviceId', options: Options(headers: {'Authorization': 'Bearer $deviceToken'}));
    } on DioException catch (e) {
      throw _translate(e);
    }
  }
}
