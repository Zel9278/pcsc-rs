package io.github.zel9278.pcscrs;

// Runs shell commands as the shell user, in the process Shizuku starts for this app
interface IShell {
    // Shizuku calls this transaction code when it stops the service
    void destroy() = 16777114;

    // The exit code on the first line, then stdout and stderr together
    String exec(String command) = 1;
}
