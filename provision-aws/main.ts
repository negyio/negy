import { Construct } from "constructs";
import { App, TerraformStack, CloudBackend, NamedCloudWorkspace } from "cdktf";
import { AwsProvider } from "@cdktf/provider-aws/lib/provider";
import * as aws from "@cdktf/provider-aws/lib";
import { Network, NegyService, L7Lb, L4Lb } from "./lib";

class Infra extends Construct {
  public readonly network: Network;
  public readonly cluster: aws.ecsCluster.EcsCluster;

  constructor(scope: Construct, name: string) {
    super(scope, name);

    const network = new Network(this, "NegyNetwork");
    const cluster = new aws.ecsCluster.EcsCluster(this, "NegyCluster", {
      name: "NegyCluster",
    });

    this.network = network;
    this.cluster = cluster;
  }
}

interface InfraCommonProps {
  network: Network;
  cluster: aws.ecsCluster.EcsCluster;
}

interface NodeProps extends InfraCommonProps {
  tasks: number;
}

class Node extends Construct {
  constructor(scope: Construct, name: string, props: NodeProps) {
    super(scope, name);

    new NegyService(this, "NegyNode", {
      cluster: props.cluster,
      vpc: props.network.vpc,
      subnets: props.network.publicSubnets,
      image: "tbrand/negy-node:latest",
      port: 3000,
      cpu: 256,
      memory: 512,
      command: ["--node-pool-endpoint", "https://pool.negy.io"],
      tasks: props.tasks,
      isPublic: true,
    });
  }
}

interface NodePoolProps extends InfraCommonProps {}

class NodePool extends Construct {
  constructor(scope: Construct, name: string, props: NodePoolProps) {
    super(scope, name);

    const lb = new L7Lb(this, "NegyNodePoolLb", {
      vpc: props.network.vpc,
      subnets: props.network.publicSubnets,
      containerPort: 3030,
      listenerPort: 80,
      healthCheck: {
        enabled: true,
        path: "/ping",
      },
    });

    new NegyService(this, "NegyNodePool", {
      cluster: props.cluster,
      vpc: props.network.vpc,
      subnets: props.network.publicSubnets,
      image: "tbrand/negy-node-pool:latest",
      port: 3030,
      cpu: 256,
      memory: 512,
      tasks: 1,
      isPublic: true,
      loadBalancer: [
        {
          containerPort: 3030,
          containerName: "NegyNodePool",
          targetGroupArn: lb.targetGroup.arn,
        },
      ],
    });
  }
}

interface GatewayProps extends InfraCommonProps {
  tasks: number;
}

class Gateway extends Construct {
  constructor(scope: Construct, name: string, props: GatewayProps) {
    super(scope, name);

    const lb = new L4Lb(this, "NegyGatewayLb", {
      vpc: props.network.vpc,
      subnets: props.network.publicSubnets,
      containerPort: 3000,
      listenerPort: 1080,
    });

    new NegyService(this, "NegyGateway", {
      cluster: props.cluster,
      vpc: props.network.vpc,
      subnets: props.network.publicSubnets,
      image: "tbrand/negy-gateway:latest",
      port: 3000,
      cpu: 256,
      memory: 512,
      command: ["--node-pool-endpoint", "https://pool.negy.io", "--hops", "3"],
      tasks: 1,
      isPublic: true,
      loadBalancer: [
        {
          containerPort: 3000,
          containerName: "NegyGateway",
          targetGroupArn: lb.targetGroup.arn,
        }
      ],
    });
  }
}

class MainStack extends TerraformStack {
  constructor(scope: Construct, name: string) {
    super(scope, name);

    new AwsProvider(this, "AWS", {
      region: "us-east-1",
    });

    const infra = new Infra(this, "NegyInfra");

    new Node(this, "NegyNode", {
      network: infra.network,
      cluster: infra.cluster,
      tasks: 3,
    });

    new NodePool(this, "NegyNodePool", {
      network: infra.network,
      cluster: infra.cluster,
    });

    new Gateway(this, "NegyGateway", {
      network: infra.network,
      cluster: infra.cluster,
      tasks: 3,
    });

    new CloudBackend(this, {
      hostname: "app.terraform.io",
      organization: "negy",
      workspaces: new NamedCloudWorkspace("negy"),
    });
  }
}

const app = new App();

new MainStack(app, "NeyStack");

app.synth();
