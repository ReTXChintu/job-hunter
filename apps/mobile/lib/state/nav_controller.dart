import 'package:flutter/foundation.dart';

import '../logic/filters.dart';
import '../logic/links.dart';

/// The signed-in shell's selected tab, plus one-shot requests for a tab to
/// open with a particular filter (e.g. Home's "Pending approval" count opens
/// Applications filtered to it).
class NavController extends ChangeNotifier {
  int tab = tabHome;
  ApplicationBucket? requestedBucket;
  JobFilter? requestedJobFilter;

  void select(int index) {
    if (tab == index) return;
    tab = index;
    notifyListeners();
  }

  void openApplications([ApplicationBucket? bucket]) {
    requestedBucket = bucket;
    tab = tabApplications;
    notifyListeners();
  }

  void openJobs([JobFilter? filter]) {
    requestedJobFilter = filter;
    tab = tabJobs;
    notifyListeners();
  }

  ApplicationBucket? takeBucket() {
    final b = requestedBucket;
    requestedBucket = null;
    return b;
  }

  JobFilter? takeJobFilter() {
    final f = requestedJobFilter;
    requestedJobFilter = null;
    return f;
  }
}
