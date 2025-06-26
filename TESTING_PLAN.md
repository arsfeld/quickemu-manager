# Dioxus App Integration Testing Plan

### 1. **Objective**
Create a suite of integration tests for the `dioxus-app` to ensure the core user-facing features are working correctly. These tests will simulate real user interactions in a browser environment.

### 2. **Technology Choice**
We will use **Playwright** for end-to-end testing. It's a modern, capable framework that allows for robust browser automation and is well-suited for testing Dioxus applications. It will be managed as a `dev-dependency` within the `dioxus-app` directory.

### 3. **Test Scenarios**
The tests will cover the following key user stories, derived from the product guidelines:

*   **Test Case 1: VM Discovery and Listing**
    *   **Goal:** Verify that the main page loads and correctly displays the list of available virtual machines.
    *   **Steps:**
        1.  Start the `dioxus-app` development server.
        2.  Launch a browser and navigate to the application's URL.
        3.  Assert that the page title is "Quickemu Manager".
        4.  Assert that one or more "VM cards" are rendered on the page.
        5.  Assert that each VM card displays essential information: VM name and status.

*   **Test Case 2: VM State Management**
    *   **Goal:** Verify that users can start and stop a virtual machine from the UI.
    *   **Steps:**
        1.  Navigate to the main page.
        2.  Locate a VM card that is currently "Stopped".
        3.  Click the "Start" button for that VM.
        4.  Assert that the UI updates to show the VM's status as "Running".
        5.  Click the "Stop" button for the same VM.
        6.  Assert that the UI updates to show the VM's status as "Stopped".

*   **Test Case 3: UI Responsiveness (Future)**
    *   **Goal:** Ensure the UI is usable on different screen sizes.
    *   **Steps:**
        1.  Test the layout on a desktop viewport.
        2.  Test the layout on a mobile viewport.
        3.  Assert that the VM cards stack correctly on mobile.

### 4. **Implementation Details**
*   **Location:** Tests will be located in a new `dioxus-app/tests` directory.
*   **Setup:**
    1.  Create a `package.json` inside `dioxus-app`.
    2.  Install `playwright` and `@playwright/test`.
    3.  Configure Playwright to run against the `dx serve` development server.
*   **Execution:** The tests will be runnable via a simple `npm test` command from within the `dioxus-app` directory.
