fn main() {
    // Link the VC runtime statically while keeping the Universal CRT dynamic.
    // This keeps the installer's `tvpnd.exe --install` working on a clean
    // Windows VM without the VC++ redistributable, which otherwise blocks the
    // unattended installer with a GUI error dialog.
    static_vcruntime::metabuild();
}
