
<h1 align="center">kcursor</h1>
<p align="center">a rust implementation of the kde svg cursor format</p>

this is a rust implementation of the new kde svg cursor format, as described [in this blog post](https://blog.vladzahorodnii.com/2024/10/06/svg-cursors-everything-that-you-need-to-know-about-them/) by Vlad Zahorodnii.

as the format currently does not yet have an actual spec, this implementation currently tries to guess what makes the most sense for this format, holding itself relatively close to both the [`xcursor` crate](https://crates.io/crates/xcursor) (that it also uses to support legacy themes without scalable cursors) and the implementation in [`kwin`](https://invent.kde.org/plasma/kwin).

i also wrote a [hyprcursor rust crate](https://github.com/m4rch3n1ng/hyprcursor-rs).
