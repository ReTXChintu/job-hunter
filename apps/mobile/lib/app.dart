import 'package:flutter/material.dart';
import 'package:go_router/go_router.dart';
import 'package:provider/provider.dart';

import 'screens/home_screen.dart';
import 'screens/login_screen.dart';
import 'state/auth_controller.dart';
import 'theme/theme.dart';

/// Routes on [AuthController.status] alone: signed out shows the
/// sign-in/pairing screen, signed in shows the review shell. There is no
/// public route that skips this -- every screen past the login screen
/// assumes a live device token.
class JobHunterApp extends StatefulWidget {
  const JobHunterApp({super.key});

  @override
  State<JobHunterApp> createState() => _JobHunterAppState();
}

class _JobHunterAppState extends State<JobHunterApp> {
  late final GoRouter _router;

  @override
  void initState() {
    super.initState();
    final auth = context.read<AuthController>();
    _router = GoRouter(
      refreshListenable: auth,
      redirect: (context, state) {
        if (auth.status == AuthStatus.unknown) return null;
        final signedIn = auth.status == AuthStatus.signedIn;
        final onLogin = state.matchedLocation == '/login';
        if (!signedIn && !onLogin) return '/login';
        if (signedIn && onLogin) return '/';
        return null;
      },
      routes: [
        GoRoute(path: '/', builder: (context, state) => const HomeScreen()),
        GoRoute(path: '/login', builder: (context, state) => const LoginScreen()),
      ],
    );
  }

  @override
  Widget build(BuildContext context) {
    final auth = context.watch<AuthController>();

    if (auth.status == AuthStatus.unknown) {
      return MaterialApp(
        debugShowCheckedModeBanner: false,
        theme: buildTheme(Brightness.light),
        darkTheme: buildTheme(Brightness.dark),
        home: const Scaffold(body: Center(child: CircularProgressIndicator())),
      );
    }

    return MaterialApp.router(
      title: 'Job Hunter',
      debugShowCheckedModeBanner: false,
      theme: buildTheme(Brightness.light),
      darkTheme: buildTheme(Brightness.dark),
      routerConfig: _router,
    );
  }
}
