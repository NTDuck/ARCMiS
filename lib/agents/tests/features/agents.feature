Feature: Agent registry
  Agents register by name. Tests find agents by name.

  Scenario: Register an agent
    Given an empty registry
    When I register the agent "main"
    Then the registry holds 1 agent
    And the agent "main" is present

  Scenario: Lookup misses before registration
    Given an empty registry
    Then the agent "main" is absent
