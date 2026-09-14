Feature: Harness wiring
  The binary entry point wires the agent registry and the tool catalog.

  Scenario: Harness components resolve
    Given the tools catalog holds "echo"
    And the agents registry holds "main"
    Then the harness wiring succeeds
