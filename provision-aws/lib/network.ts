import { Construct } from "constructs";
import * as aws from "@cdktf/provider-aws/lib";

export class Network extends Construct {
  public readonly vpc: aws.vpc.Vpc;
  public readonly publicSubnets: aws.subnet.Subnet[];

  constructor(scope: Construct, name: string) {
    super(scope, name);

    const vpc = new aws.vpc.Vpc(this, `Vpc_${name}`, {
      cidrBlock: "10.0.0.0/16",
    });

    const publicSubnet1 = new aws.subnet.Subnet(this, `PublicSubnet1_${name}`, {
      vpcId: vpc.id,
      cidrBlock: "10.0.1.0/24",
      availabilityZone: "us-east-1a",
      mapPublicIpOnLaunch: true,
    });

    const publicSubnet2 = new aws.subnet.Subnet(this, `PublicSubnet2_${name}`, {
      vpcId: vpc.id,
      cidrBlock: "10.0.2.0/24",
      availabilityZone: "us-east-1b",
      mapPublicIpOnLaunch: true,
    });

    const internetGateway = new aws.internetGateway.InternetGateway(this, `InternetGateway_${name}`, {
      vpcId: vpc.id,
    });

    const routeTablePublic = new aws.routeTable.RouteTable(this, `NegyRouteTablePublic_${name}`, {
      vpcId: vpc.id,
    });

    new aws.route.Route(this, `RoutePublic_${name}`, {
      routeTableId: routeTablePublic.id,
      gatewayId: internetGateway.id,
      destinationCidrBlock: "0.0.0.0/0",
    });

    new aws.routeTableAssociation.RouteTableAssociation(this, `RouteTableAssociationPublic1_${name}`, {
      routeTableId: routeTablePublic.id,
      subnetId: publicSubnet1.id,
    });

    new aws.routeTableAssociation.RouteTableAssociation(this, `RouteTableAssociationPublic2_${name}`, {
      routeTableId: routeTablePublic.id,
      subnetId: publicSubnet2.id,
    });

    this.vpc = vpc;
    this.publicSubnets = [publicSubnet1, publicSubnet2];
  }
}
