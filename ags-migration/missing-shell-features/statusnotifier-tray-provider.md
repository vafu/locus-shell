# StatusNotifier Tray Provider

## Needed By

- [Bar](../migration/widgets/bar.md)

## Gap

The bar needs tray item discovery, icons, menus, and activation behavior.

## Direction

Add a typed StatusNotifier/AppIndicator provider and DBusMenu support, likely
consumer-local until the public API stabilizes.

